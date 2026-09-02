# Schema

Markdown files remain the source of truth. A workspace does not need setup,
migrations, a database file, configuration, or a schema file.

Schema has two layers:

- inferred schema, always available from the loaded Markdown documents;
- explicit schema, optional, stored as `flokin.schema.yaml` at the workspace root.

FlokinMD never creates `flokin.schema.yaml` automatically when opening a
workspace. Creating it requires an explicit user action.

## Inferred Schema

Every Collection receives an inferred schema from the already loaded Documents.
The scanner is not rerun for schema rendering.

Supported inferred types:

- `String`
- `Integer`
- `Float`
- `Boolean`
- `Array`
- `Object`
- `Relation`
- `Mixed`
- `Null`
- `Unknown`

Relations are explicit only. A field is inferred as `Relation` when the
`RelationIndex` sees wikilinks such as `owner: "[[Sergio]]"` for that property.
Plain strings such as `owner: Sergio` remain `String`.

Missing and null are distinct. A missing property does not count as present.
An explicit `status: null` counts as present and makes the field nullable.

A field is required only when it is present in every document in the Collection.
The schema keeps coverage as `present / total`.

`title` is structural. It appears once as `String required`, using the resolved
Document title from frontmatter title, first H1, or filename.

Fields are ordered deterministically: `title` first, then alphabetical.

## Explicit Schema

A workspace may optionally include this file at the workspace root:

```yaml
version: 1

collections:
  projects:
    fields:
      title:
        type: string
        required: true

      status:
        type: string
        required: true

      priority:
        type: integer
        required: false

      published:
        type: boolean
        required: false

      owner:
        type: relation
        required: false
```

Supported explicit field types:

- `string`
- `integer`
- `float`
- `boolean`
- `array`
- `object`
- `relation`
- `mixed`

`version` may be omitted and is treated as `1`. Incompatible versions are not
interpreted silently.

Invalid schema files do not block the workspace; inferred schema remains
available, Schema View shows a warning, and Database Health reports the schema
issue.

## Creating A Schema

When no explicit schema exists, Schema View shows `Schema inferido` and offers
`Criar schema explícito`. Database Health can show the same creation action as a
compact hint.

The creation dialog explains that FlokinMD will create:

```text
<workspace>/flokin.schema.yaml
```

The generated file is a snapshot of the currently inferred schema:

- all non-empty Collections are included;
- fields use the current inferred type;
- `required` uses the current coverage observation;
- output order is deterministic.

Mixed, Null, and Unknown fields are not declared automatically because FlokinMD
does not guess a definitive type. Mixed fields are listed in the dialog and can
be added manually after the data or schema is clarified.

If `flokin.schema.yaml` already exists, FlokinMD does not overwrite it. The UI
offers `Abrir schema` instead.

## Editing A Schema

When an explicit schema exists, Schema View shows `Schema explícito` and an
`Abrir schema` action. The same action appears in Database Health when the file
is invalid.

`flokin.schema.yaml` opens in the real editor as a special configuration tab:

- it is not scanned as a Markdown Document;
- it supports editing, dirty state, close protection, and Ctrl+S;
- saving writes only the schema file;
- the watcher reloads SchemaCatalog and Database Health after save.

If the edit makes the YAML invalid, the editor remains usable, inferred schema
continues to work, and Health reports the error. Correcting and saving the file
removes the Health issue through the same reload pipeline.
