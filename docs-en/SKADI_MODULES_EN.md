# Files, Imports, and Visibility

```skadi
import "./math.skd"

new Int result = math.add(2, 3)
new math.Point origin = {x = 0.0, y = 0.0}
```

The qualifier is the file name without `.skd`. A file sees public symbols only
from modules it imports directly. Transitive imports are not implicit
re-exports.

Top-level functions, structs, and labels are public by default. Prefix them with
`local` to keep them in the file.

Named package imports, aliases, re-exports, and dependency resolution are not
implemented.
