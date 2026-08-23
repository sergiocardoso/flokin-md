# Design System

MDB-001 defines a small token-based design system in `src/App.css`.

## Colors

- `--color-background`
- `--color-surface`
- `--color-surface-muted`
- `--color-border`
- `--color-text`
- `--color-text-muted`
- `--color-primary`
- `--color-primary-hover`
- `--color-success`
- `--color-warning`
- `--color-danger`

## Spacing

- `--space-4`
- `--space-8`
- `--space-12`
- `--space-16`
- `--space-20`
- `--space-24`
- `--space-32`

## Radius

- `--radius-small`
- `--radius-medium`
- `--radius-large`

## Typography

- `--font-display`
- `--font-heading`
- `--font-body`
- `--font-label`
- `--font-mono`

## Usage Rules

New screens should reuse these tokens before introducing new values. If a new token is needed, document why it belongs in the shared system instead of hard-coding one-off CSS.

The initial visual direction is a light desktop app with clean surfaces, subtle borders, rounded panels, good information density, and a purple primary accent.
