# Schema

Schema in FlokinMD is read-only in MDB-014.

Markdown files remain the source of truth. A workspace does not need setup,
migrations, a database file, configuration, or a schema file.

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

FlokinMD never creates, formats, saves, or migrates `flokin.schema.yaml` in
MDB-014. Invalid schema files do not block the workspace; inferred schema
remains available and the UI shows a warning.
