use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

use flokin_core::{should_ignore_workspace_path, WorkspaceEvent};
use iced::{stream, Subscription};
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};

use crate::message::Message;

const DEBOUNCE_WINDOW: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherMessage {
    Events {
        workspace: PathBuf,
        events: Vec<WorkspaceEvent>,
    },
    Failed {
        workspace: PathBuf,
        message: String,
    },
}

#[derive(Debug)]
enum RawWatcherMessage {
    Event(Event),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PendingAction {
    Upsert,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WatchedWorkspace(PathBuf);

pub fn subscription(workspace: Option<PathBuf>) -> Subscription<Message> {
    match workspace {
        Some(workspace) => Subscription::run_with(WatchedWorkspace(workspace), watch_workspace)
            .map(Message::WorkspaceWatcher),
        None => Subscription::none(),
    }
}

fn watch_workspace(
    workspace: &WatchedWorkspace,
) -> impl iced::futures::Stream<Item = WatcherMessage> {
    let workspace = workspace.0.clone();

    stream::channel(16, async move |mut output| {
        thread::spawn(move || {
            let (sender, receiver) = mpsc::channel::<RawWatcherMessage>();
            let watcher_workspace = workspace.clone();
            let callback_sender = sender.clone();
            let mut watcher = match RecommendedWatcher::new(
                move |result: notify::Result<Event>| {
                    let message = match result {
                        Ok(event) => RawWatcherMessage::Event(event),
                        Err(error) => RawWatcherMessage::Error(error.to_string()),
                    };
                    let _ = callback_sender.send(message);
                },
                Config::default(),
            ) {
                Ok(watcher) => watcher,
                Err(error) => {
                    let _ = sender.send(RawWatcherMessage::Error(error.to_string()));
                    return;
                }
            };

            if let Err(error) = watcher.watch(&watcher_workspace, RecursiveMode::Recursive) {
                let _ = sender.send(RawWatcherMessage::Error(error.to_string()));
            }

            let mut pending = Vec::<Event>::new();

            loop {
                match receiver.recv() {
                    Ok(RawWatcherMessage::Event(event)) => pending.push(event),
                    Ok(RawWatcherMessage::Error(message)) => {
                        if output
                            .try_send(WatcherMessage::Failed {
                                workspace: watcher_workspace.clone(),
                                message,
                            })
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    }
                    Err(_) => break,
                }

                while let Ok(message) = receiver.recv_timeout(DEBOUNCE_WINDOW) {
                    match message {
                        RawWatcherMessage::Event(event) => pending.push(event),
                        RawWatcherMessage::Error(message) => {
                            if output
                                .try_send(WatcherMessage::Failed {
                                    workspace: watcher_workspace.clone(),
                                    message,
                                })
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                }

                let events = coalesce_events(&watcher_workspace, pending.drain(..));
                if !events.is_empty()
                    && output
                        .try_send(WatcherMessage::Events {
                            workspace: watcher_workspace.clone(),
                            events,
                        })
                        .is_err()
                {
                    break;
                }
            }
        });
    })
}

fn coalesce_events(
    workspace: &Path,
    events: impl IntoIterator<Item = Event>,
) -> Vec<WorkspaceEvent> {
    let mut renames = Vec::<WorkspaceEvent>::new();
    let mut actions = BTreeMap::<PathBuf, PendingAction>::new();

    for event in events {
        if let Some(rename) = rename_event(workspace, &event) {
            renames.push(rename);
            continue;
        }

        for path in event.paths {
            if should_ignore_workspace_path(workspace, &path) {
                continue;
            }

            match event.kind {
                EventKind::Create(CreateKind::Any | CreateKind::File)
                | EventKind::Modify(
                    ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Metadata(_),
                )
                | EventKind::Modify(ModifyKind::Name(RenameMode::To | RenameMode::Any))
                | EventKind::Modify(ModifyKind::Other) => {
                    actions.insert(path, PendingAction::Upsert);
                }
                EventKind::Remove(RemoveKind::Any | RemoveKind::File)
                | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    actions.insert(path, PendingAction::Remove);
                }
                EventKind::Remove(RemoveKind::Folder) | EventKind::Create(CreateKind::Folder) => {
                    actions.insert(path, PendingAction::Upsert);
                }
                EventKind::Remove(RemoveKind::Other)
                | EventKind::Create(CreateKind::Other)
                | EventKind::Modify(_) => {
                    actions.insert(path, PendingAction::Upsert);
                }
                EventKind::Access(_) | EventKind::Any | EventKind::Other => {}
            }
        }
    }

    let renamed_from = renames
        .iter()
        .filter_map(|event| match event {
            WorkspaceEvent::Rename { from, .. } => Some(from.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let renamed_to = renames
        .iter()
        .filter_map(|event| match event {
            WorkspaceEvent::Rename { to, .. } => Some(to.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    let mut normalized = renames;
    normalized.extend(actions.into_iter().filter_map(|(path, action)| {
        if renamed_from.contains(&path) || renamed_to.contains(&path) {
            return None;
        }

        Some(match action {
            PendingAction::Upsert => WorkspaceEvent::Upsert(path),
            PendingAction::Remove => WorkspaceEvent::Remove(path),
        })
    }));

    normalized
}

fn rename_event(workspace: &Path, event: &Event) -> Option<WorkspaceEvent> {
    if !matches!(
        event.kind,
        EventKind::Modify(ModifyKind::Name(
            RenameMode::Both | RenameMode::Any | RenameMode::To
        ))
    ) || event.paths.len() < 2
    {
        return None;
    }

    let from = event.paths.first()?.clone();
    let to = event.paths.get(1)?.clone();
    if should_ignore_workspace_path(workspace, &from)
        && should_ignore_workspace_path(workspace, &to)
    {
        return None;
    }

    Some(WorkspaceEvent::Rename { from, to })
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::DataChange;

    #[test]
    fn debounce_keeps_last_action_for_path() {
        let workspace = PathBuf::from("/workspace");
        let path = workspace.join("task.md");

        let events = coalesce_events(
            &workspace,
            [
                event(
                    EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                    path.clone(),
                ),
                event(
                    EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                    path.clone(),
                ),
                event(
                    EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                    path.clone(),
                ),
            ],
        );

        assert_eq!(events, vec![WorkspaceEvent::Upsert(path)]);
    }

    #[test]
    fn non_markdown_is_preserved_for_core_filtering() {
        let workspace = PathBuf::from("/workspace");
        let path = workspace.join("notes.txt");

        let events = coalesce_events(
            &workspace,
            [event(
                EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                path.clone(),
            )],
        );

        assert_eq!(events, vec![WorkspaceEvent::Upsert(path)]);
    }

    #[test]
    fn ignored_directories_are_dropped() {
        let workspace = PathBuf::from("/workspace");

        let events = coalesce_events(
            &workspace,
            [
                event(
                    EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                    workspace.join(".git/config.md"),
                ),
                event(
                    EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                    workspace.join("target/out.md"),
                ),
                event(
                    EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                    workspace.join("node_modules/pkg/readme.md"),
                ),
            ],
        );

        assert!(events.is_empty());
    }

    #[test]
    fn rename_event_does_not_duplicate_paths() {
        let workspace = PathBuf::from("/workspace");
        let from = workspace.join("projects/carf.md");
        let to = workspace.join("projects/carf-2026.md");

        let events = coalesce_events(
            &workspace,
            [Event {
                kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                paths: vec![from.clone(), to.clone()],
                attrs: Default::default(),
            }],
        );

        assert_eq!(events, vec![WorkspaceEvent::Rename { from, to }]);
    }

    fn event(kind: EventKind, path: PathBuf) -> Event {
        Event {
            kind,
            paths: vec![path],
            attrs: Default::default(),
        }
    }
}
