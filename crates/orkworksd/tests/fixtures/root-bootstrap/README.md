# Root bootstrap run bytes

`run.json` is independently extracted from the existing Python-derived root catalog
using Python standard-library JSON (sorted keys, compact separators, UTF-8, no
terminal newline). Its SHA-256 is `92dafe64fe728ce3feb6b8c58e5bcce453f612f6c36c1688c53707893ce69851`.
The existing runDefinitionDigest remains `dcd834cf0cd1ad11906b3547a97ffc74b16c039f7f6c59f72a3f3c636f2f5081`.
No Rust validator or serializer generated these expectations. The root bootstrap,
render and artifact expectations remain in docs/validation/root-bootstrap-vectors.json.
