#

<!-- TODO add Abort answer? -->
```mermaid
sequenceDiagram
    title Get all data from plugin
    participant runner
    participant plugin

    runner ->> plugin: GetData(entry_text)
    plugin ->> runner: Ok
    loop Data transfer
        plugin ->> runner: Hit 0
        runner ->> plugin: Ok
        plugin ->> runner: Hit 1
        runner ->> plugin: Err
        plugin ->> runner: Hit 1
        runner ->> plugin: Ok
        plugin ->> runner: Hit 2
        runner ->> plugin: Ok
        plugin ->> runner: Abort
    end
```

```mermaid
sequenceDiagram
    title Break data receiving with new query
    participant runner
    participant plugin

    runner ->> plugin: GetData(entry_text)
    plugin ->> runner: Ok
    loop Data transfer
        plugin ->> runner: Hit
        runner ->> plugin: Ok
        plugin ->> runner: Hit
        runner ->> plugin: Ok
        plugin ->> runner: Hit
        runner ->> plugin: Abort
    end
    runner ->> plugin: GetData(new_entry_text)
```

```mermaid
sequenceDiagram
    title Choose what to launch
    participant runner
    participant plugin

    runner ->> plugin: Activate(UUID)
    plugin ->> runner: Ok
    runner ->> plugin: Activate(UUID)
    plugin ->> runner: Err
```

```mermaid
sequenceDiagram
    title On Quit
    participant runner
    participant plugin

    runner ->> plugin: Quit
    Note over plugin: Plugin do what it need to deinit
    plugin ->> runner: Ok|Err
    Note over runner: Waiting for result but don't really cares what it will be
```
