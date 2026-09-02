use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use flokin_core::{
    GraphEdge, GraphEdgeStatus, GraphNode, GraphNodeId, GraphNodeKind, GraphPoint, GraphProjection,
};
use iced::widget::{button, canvas, column, container, row, scrollable, text};
use iced::{
    alignment, border, mouse, Color, Element, Length, Pixels, Point, Rectangle, Renderer, Size,
    Theme, Vector,
};

use crate::{message::Message, theme, widgets};

const NODE_WIDTH: f32 = theme::sizes::GRAPH_NODE_WIDTH;
const NODE_HEIGHT: f32 = theme::sizes::GRAPH_NODE_HEIGHT;
const FIT_PADDING: f32 = 72.0;
const NODE_PADDING_X: f32 = 13.0;

#[derive(Debug, Clone)]
pub struct GraphViewState {
    pub projection: GraphProjection,
    pub positions: BTreeMap<GraphNodeId, GraphPoint>,
    pub pan: Vector,
    pub zoom: f32,
    pub viewport: Size,
}

impl Default for GraphViewState {
    fn default() -> Self {
        Self {
            projection: GraphProjection {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            positions: BTreeMap::new(),
            pan: Vector::new(FIT_PADDING, FIT_PADDING),
            zoom: 1.0,
            viewport: Size::ZERO,
        }
    }
}

pub fn sidebar(state: &GraphViewState, width: f32) -> Element<'_, Message> {
    let problems = state.projection.problem_count();
    let mut content = column![
        widgets::section_title("GRAPH"),
        graph_metric("Documents", state.projection.document_count()),
        graph_metric("Relations", state.projection.relation_count()),
    ]
    .spacing(theme::spacing::SM);

    if problems > 0 {
        content = content.push(graph_metric("Problems", problems));
    }

    let mut collections = state
        .projection
        .nodes
        .iter()
        .filter_map(|node| node.collection.as_deref())
        .fold(BTreeMap::<&str, usize>::new(), |mut counts, collection| {
            *counts.entry(collection).or_default() += 1;
            counts
        });

    if !collections.is_empty() {
        let mut list = column![widgets::section_title("COLLECTIONS")].spacing(theme::spacing::XS);
        for (collection, count) in std::mem::take(&mut collections) {
            list = list.push(
                row![
                    text(collection)
                        .size(theme::typography::BODY)
                        .width(Length::Fill),
                    text(count.to_string())
                        .size(theme::typography::LABEL)
                        .style(theme::text_muted),
                ]
                .align_y(iced::Alignment::Center),
            );
        }
        content = content.push(container(list).padding([theme::spacing::MD, 0.0]));
    }

    container(scrollable(content))
        .width(width)
        .height(Length::Fill)
        .padding(theme::spacing::MD)
        .style(theme::panel)
        .into()
}

fn graph_metric(label: &'static str, value: usize) -> Element<'static, Message> {
    row![
        text(label)
            .size(theme::typography::BODY)
            .width(Length::Fill),
        text(value.to_string())
            .size(theme::typography::BODY)
            .font(theme::mono())
            .style(theme::text_muted),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn view<'a>(
    state: &'a GraphViewState,
    selected_document: Option<&'a std::path::PathBuf>,
) -> Element<'a, Message> {
    let summary = format!(
        "{} documentos • {} relações",
        state.projection.document_count(),
        state.projection.relation_count()
    );
    let zoom_label = zoom_label(state.zoom);

    let zoom_controls = toolbar_group(row![
        graph_icon_button(
            theme::Icon::Minus,
            "Diminuir zoom",
            Some(Message::GraphZoomOut),
        ),
        graph_icon_button(
            theme::Icon::Plus,
            "Aumentar zoom",
            Some(Message::GraphZoomIn),
        ),
        container(
            text(zoom_label)
                .size(theme::typography::LABEL)
                .font(theme::mono())
                .style(graph_zoom_text)
        )
        .width(theme::sizes::GRAPH_ZOOM_BADGE_WIDTH)
        .height(theme::sizes::GRAPH_TOOLBAR_BUTTON_SIZE)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(theme::graph_zoom_badge),
        graph_icon_button(
            theme::Icon::Reset,
            "Resetar zoom",
            Some(Message::GraphZoomReset),
        ),
    ]);

    let navigation_controls = toolbar_group(row![
        graph_icon_button(
            theme::Icon::Focus,
            "Centralizar selecionado",
            selected_document.map(|_| Message::GraphFocusSelected),
        ),
        graph_icon_button(
            theme::Icon::Frame,
            "Enquadrar grafo",
            Some(Message::GraphFitRequested),
        ),
    ]);

    let toolbar = row![
        text("Grafo")
            .size(theme::typography::TITLE)
            .style(theme::text_normal),
        text(summary)
            .size(theme::typography::BODY)
            .style(theme::text_muted),
        iced::widget::Space::new().width(Length::Fill),
        zoom_controls,
        navigation_controls,
    ]
    .spacing(theme::spacing::MD)
    .align_y(iced::Alignment::Center);

    let canvas = canvas(GraphCanvas {
        projection: &state.projection,
        positions: &state.positions,
        pan: state.pan,
        zoom: state.zoom,
        selected_document,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    column![
        container(toolbar)
            .height(theme::sizes::TOOLBAR_HEIGHT)
            .padding([0.0, theme::spacing::LG])
            .style(theme::top_bar),
        container(canvas)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::graph_panel)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn zoom_label(zoom: f32) -> String {
    format!("{:.0}%", (zoom * 100.0).round())
}

fn toolbar_group<'a>(content: iced::widget::Row<'a, Message>) -> Element<'a, Message> {
    container(content.spacing(theme::spacing::XS))
        .padding(2.0)
        .style(theme::graph_toolbar_group)
        .into()
}

fn graph_icon_button<'a>(
    icon: theme::Icon,
    tooltip: &'static str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let mut control = button(widgets::icon(icon, theme::icons::TOOLBAR, false))
        .width(theme::sizes::GRAPH_TOOLBAR_BUTTON_SIZE)
        .height(theme::sizes::GRAPH_TOOLBAR_BUTTON_SIZE)
        .padding(0)
        .style(theme::button_graph_toolbar);
    if let Some(message) = on_press {
        control = control.on_press(message);
    }

    iced::widget::tooltip(
        control,
        container(text(tooltip).size(theme::typography::LABEL))
            .padding([4.0, 7.0])
            .style(theme::overlay_panel),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}

fn graph_zoom_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme::palette(theme).graph_zoom_badge_text),
    }
}

struct GraphCanvas<'a> {
    projection: &'a GraphProjection,
    positions: &'a BTreeMap<GraphNodeId, GraphPoint>,
    pan: Vector,
    zoom: f32,
    selected_document: Option<&'a std::path::PathBuf>,
}

#[derive(Debug, Default)]
struct CanvasState {
    drag: Option<DragState>,
    last_click: Option<(GraphNodeId, Instant)>,
}

#[derive(Debug)]
enum DragState {
    Background { last: Point },
    Node { node: GraphNodeId, last: Point },
}

impl canvas::Program<Message> for GraphCanvas<'_> {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let cursor = cursor.position_in(bounds)?;
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(node) = self.hit_node(cursor) {
                    state.drag = Some(DragState::Node { node, last: cursor });
                } else {
                    state.drag = Some(DragState::Background { last: cursor });
                }
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => match state.drag.as_mut() {
                Some(DragState::Background { last }) => {
                    let dx = cursor.x - last.x;
                    let dy = cursor.y - last.y;
                    *last = cursor;
                    Some(canvas::Action::publish(Message::GraphPanBy(dx, dy)).and_capture())
                }
                Some(DragState::Node { node, last }) => {
                    let dx = (cursor.x - last.x) / self.zoom;
                    let dy = (cursor.y - last.y) / self.zoom;
                    *last = cursor;
                    Some(
                        canvas::Action::publish(Message::GraphNodeDragged {
                            node: node.clone(),
                            dx,
                            dy,
                        })
                        .and_capture(),
                    )
                }
                None => Some(canvas::Action::request_redraw()),
            },
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let clicked = match state.drag.take() {
                    Some(DragState::Node { node, last }) if distance(last, cursor) < 3.0 => {
                        Some(node)
                    }
                    _ => None,
                };
                if let Some(node) = clicked {
                    let now = Instant::now();
                    let double = state
                        .last_click
                        .as_ref()
                        .is_some_and(|(last_node, last_at)| {
                            last_node == &node
                                && now.duration_since(*last_at) <= Duration::from_millis(450)
                        });
                    state.last_click = Some((node.clone(), now));
                    let message = if double {
                        Message::GraphNodeOpened(node)
                    } else {
                        Message::GraphNodeSelected(node)
                    };
                    Some(canvas::Action::publish(message).and_capture())
                } else {
                    Some(canvas::Action::capture())
                }
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let delta = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y * 0.12,
                    mouse::ScrollDelta::Pixels { y, .. } => *y * 0.002,
                };
                Some(
                    canvas::Action::publish(Message::GraphZoomAt {
                        x: cursor.x,
                        y: cursor.y,
                        delta,
                    })
                    .and_capture(),
                )
            }
            canvas::Event::Window(_) => Some(canvas::Action::publish(
                Message::GraphViewportChanged(bounds.width, bounds.height),
            )),
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let palette = theme::palette(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), palette.graph_background);
        draw_grid(&mut frame, palette.graph_canvas_grid);

        let hover = cursor
            .position_in(bounds)
            .and_then(|position| self.hit_node(position));

        frame.with_save(|frame| {
            frame.translate(self.pan);
            frame.scale(self.zoom);
            draw_edges(frame, self, &hover, theme);
            draw_nodes(frame, self, &hover, theme);
        });

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if cursor
            .position_in(bounds)
            .and_then(|position| self.hit_node(position))
            .is_some()
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::Grab
        }
    }
}

impl GraphCanvas<'_> {
    fn hit_node(&self, screen: Point) -> Option<GraphNodeId> {
        let world = Point::new(
            (screen.x - self.pan.x) / self.zoom,
            (screen.y - self.pan.y) / self.zoom,
        );
        self.projection.nodes.iter().rev().find_map(|node| {
            let position = self.positions.get(&node.id)?;
            let rect = Rectangle {
                x: position.x,
                y: position.y,
                width: NODE_WIDTH,
                height: NODE_HEIGHT,
            };
            rect.contains(world).then(|| node.id.clone())
        })
    }

    fn is_selected_or_hovered(&self, id: &GraphNodeId, hover: &Option<GraphNodeId>) -> bool {
        hover.as_ref() == Some(id)
            || matches!((id, self.selected_document), (GraphNodeId::Document(path), Some(selected)) if path == selected)
    }

    fn is_adjacent(&self, edge: &GraphEdge, hover: &Option<GraphNodeId>) -> bool {
        let selected = self
            .selected_document
            .map(|path| GraphNodeId::Document(path.clone()));
        selected
            .as_ref()
            .is_some_and(|id| &edge.source == id || &edge.target == id)
            || hover
                .as_ref()
                .is_some_and(|id| &edge.source == id || &edge.target == id)
    }
}

fn draw_grid(frame: &mut canvas::Frame, color: Color) {
    let step = theme::sizes::GRAPH_GRID_STEP;
    let mut x = step / 2.0;
    while x <= frame.width() {
        let mut y = step / 2.0;
        while y <= frame.height() {
            frame.fill(
                &canvas::Path::circle(Point::new(x, y), 0.75),
                Color { a: 0.38, ..color },
            );
            y += step;
        }
        x += step;
    }
}

fn draw_edges(
    frame: &mut canvas::Frame,
    graph: &GraphCanvas<'_>,
    hover: &Option<GraphNodeId>,
    theme: &Theme,
) {
    let palette = theme::palette(theme);
    let mut parallel_counts = BTreeMap::<(&GraphNodeId, &GraphNodeId), usize>::new();
    for edge in &graph.projection.edges {
        *parallel_counts
            .entry((&edge.source, &edge.target))
            .or_default() += 1;
    }
    let mut parallel_index = BTreeMap::<(&GraphNodeId, &GraphNodeId), usize>::new();

    for edge in &graph.projection.edges {
        let Some(source) = graph.positions.get(&edge.source) else {
            continue;
        };
        let Some(target) = graph.positions.get(&edge.target) else {
            continue;
        };
        let count = *parallel_counts
            .get(&(&edge.source, &edge.target))
            .unwrap_or(&1);
        let index = parallel_index
            .entry((&edge.source, &edge.target))
            .or_default();
        let offset = (*index as f32 - (count.saturating_sub(1) as f32 / 2.0)) * 20.0;
        *index += 1;

        let source_center = Point::new(source.x + NODE_WIDTH / 2.0, source.y + NODE_HEIGHT / 2.0);
        let target_center = Point::new(target.x + NODE_WIDTH / 2.0, target.y + NODE_HEIGHT / 2.0);
        let active = graph.is_adjacent(edge, hover);
        let color = match edge.status {
            GraphEdgeStatus::Resolved if active => palette.graph_edge_active,
            GraphEdgeStatus::Resolved => palette.graph_edge,
            GraphEdgeStatus::Unresolved => palette.graph_unresolved,
            GraphEdgeStatus::Ambiguous => palette.graph_ambiguous,
        };

        if edge.source == edge.target {
            let loop_path = canvas::Path::new(|path| {
                let x = source.x + NODE_WIDTH - 12.0;
                let y = source.y + 6.0;
                path.move_to(Point::new(x, y + 22.0));
                path.bezier_curve_to(
                    Point::new(x + 66.0, y - 28.0),
                    Point::new(x + 72.0, y + 76.0),
                    Point::new(x, y + 50.0),
                );
            });
            frame.stroke(
                &loop_path,
                canvas::Stroke::default()
                    .with_width(if active { 1.9 } else { 1.15 })
                    .with_color(color),
            );
            draw_arrowhead(
                frame,
                Point::new(source.x + NODE_WIDTH - 12.0, source.y + 58.0),
                1.8,
                color,
            );
            if active {
                draw_edge_label(
                    frame,
                    &edge.relation_type,
                    Point::new(source.x + NODE_WIDTH + 46.0, source.y + 8.0),
                    color,
                    theme,
                );
            }
            continue;
        }

        let dx = target_center.x - source_center.x;
        let dy = target_center.y - source_center.y;
        let length = (dx * dx + dy * dy).sqrt().max(1.0);
        let nx = -dy / length;
        let ny = dx / length;
        let start = Point::new(source_center.x + nx * offset, source_center.y + ny * offset);
        let end = Point::new(target_center.x + nx * offset, target_center.y + ny * offset);
        let control = Point::new(
            (start.x + end.x) / 2.0 + nx * offset,
            (start.y + end.y) / 2.0 + ny * offset,
        );
        let path = canvas::Path::new(|path| {
            path.move_to(start);
            path.quadratic_curve_to(control, end);
        });
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_width(if active { 1.75 } else { 1.05 })
                .with_color(color),
        );
        let angle = dy.atan2(dx);
        draw_arrowhead(frame, end, angle, color);
        if active {
            draw_edge_label(frame, &edge.relation_type, control, color, theme);
        }
    }
}

fn draw_edge_label(
    frame: &mut canvas::Frame,
    label: &str,
    point: Point,
    color: Color,
    _theme: &Theme,
) {
    let palette = theme::palette(_theme);
    let label_width = 128.0;
    let label_height = 20.0;
    let background = canvas::Path::rounded_rectangle(
        Point::new(point.x - label_width / 2.0, point.y - label_height / 2.0),
        Size::new(label_width, label_height),
        border::Radius::from(theme::radius::SM),
    );
    frame.fill(
        &background,
        Color {
            a: 0.88,
            ..palette.graph_background
        },
    );
    frame.fill_text(canvas::Text {
        content: label.to_owned(),
        position: point,
        max_width: 140.0,
        color,
        size: Pixels(theme::sizes::GRAPH_EDGE_LABEL_FONT_SIZE as f32),
        font: theme::mono(),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Center,
        ..canvas::Text::default()
    });
}

fn draw_arrowhead(frame: &mut canvas::Frame, tip: Point, angle: f32, color: Color) {
    let size = 8.0;
    let left = Point::new(
        tip.x - size * (angle - 0.45).cos(),
        tip.y - size * (angle - 0.45).sin(),
    );
    let right = Point::new(
        tip.x - size * (angle + 0.45).cos(),
        tip.y - size * (angle + 0.45).sin(),
    );
    let arrow = canvas::Path::new(|path| {
        path.move_to(tip);
        path.line_to(left);
        path.move_to(tip);
        path.line_to(right);
    });
    frame.stroke(
        &arrow,
        canvas::Stroke::default().with_width(1.2).with_color(color),
    );
}

fn draw_nodes(
    frame: &mut canvas::Frame,
    graph: &GraphCanvas<'_>,
    hover: &Option<GraphNodeId>,
    theme: &Theme,
) {
    for node in &graph.projection.nodes {
        let Some(position) = graph.positions.get(&node.id) else {
            continue;
        };
        draw_node(frame, graph, node, *position, hover, theme);
    }
}

fn draw_node(
    frame: &mut canvas::Frame,
    graph: &GraphCanvas<'_>,
    node: &GraphNode,
    position: GraphPoint,
    hover: &Option<GraphNodeId>,
    theme: &Theme,
) {
    let palette = theme::palette(theme);
    let selected = matches!(
        (&node.id, graph.selected_document),
        (GraphNodeId::Document(path), Some(selected)) if path == selected
    );
    let hovered = hover.as_ref() == Some(&node.id);
    let background = match node.kind {
        GraphNodeKind::Document if selected => palette.graph_node_selected,
        GraphNodeKind::Document if hovered => palette.graph_node_hover,
        GraphNodeKind::Document => palette.graph_node,
        GraphNodeKind::Unresolved => Color {
            a: 0.12,
            ..palette.graph_unresolved
        },
        GraphNodeKind::Ambiguous => Color {
            a: 0.12,
            ..palette.graph_ambiguous
        },
    };
    let border_color = match node.kind {
        GraphNodeKind::Document if selected => palette.graph_edge_active,
        GraphNodeKind::Document => palette.graph_node_border,
        GraphNodeKind::Unresolved => palette.graph_unresolved,
        GraphNodeKind::Ambiguous => palette.graph_ambiguous,
    };
    let rect = canvas::Path::rounded_rectangle(
        Point::new(position.x, position.y),
        Size::new(NODE_WIDTH, NODE_HEIGHT),
        border::Radius::from(theme::radius::SM),
    );
    let shadow = canvas::Path::rounded_rectangle(
        Point::new(position.x + 0.0, position.y + 2.0),
        Size::new(NODE_WIDTH, NODE_HEIGHT),
        border::Radius::from(theme::radius::SM),
    );
    frame.fill(&shadow, palette.graph_node_shadow);
    frame.fill(&rect, background);
    frame.stroke(
        &rect,
        canvas::Stroke::default()
            .with_width(
                if selected || graph.is_selected_or_hovered(&node.id, hover) {
                    1.6
                } else {
                    1.0
                },
            )
            .with_color(border_color),
    );
    let title = fit_label(&node.title, 22);
    frame.fill_text(canvas::Text {
        content: title,
        position: Point::new(position.x + NODE_PADDING_X, position.y + 12.0),
        max_width: NODE_WIDTH - NODE_PADDING_X * 2.0,
        color: palette.text,
        size: Pixels(theme::sizes::GRAPH_NODE_FONT_SIZE as f32),
        font: theme::typography::UI,
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Top,
        ..canvas::Text::default()
    });
    let subtitle = node
        .document_type
        .as_deref()
        .or(node.collection.as_deref())
        .unwrap_or(match node.kind {
            GraphNodeKind::Document => "",
            GraphNodeKind::Unresolved => "unresolved",
            GraphNodeKind::Ambiguous => "ambiguous",
        });
    frame.fill_text(canvas::Text {
        content: fit_label(subtitle, 24),
        position: Point::new(position.x + NODE_PADDING_X, position.y + 38.0),
        max_width: NODE_WIDTH - NODE_PADDING_X * 2.0,
        color: if node.kind == GraphNodeKind::Document {
            palette.text_muted
        } else {
            border_color
        },
        size: Pixels(theme::typography::LABEL as f32),
        font: theme::typography::UI,
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Top,
        ..canvas::Text::default()
    });
}

fn fit_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        label.to_owned()
    } else {
        let mut value = label
            .chars()
            .take(max_chars.saturating_sub(1))
            .collect::<String>();
        value.push('…');
        value
    }
}

fn distance(left: Point, right: Point) -> f32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    (dx * dx + dy * dy).sqrt()
}
