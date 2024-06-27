# example

This shows how to use chook. There are three different crates involved.

## types

This defines the ArgT and RetT type that are used in both the
shim and parent binary. Just define a few types and derive
Serialize and Deserialize on them.

## shim

This is the .so file that will actually get injected into the
child binary. It must define a custom init routine which invokes
`chook_shim::run` with a custom callback.

## bin

This is the binary that actually launches a subprocess. It
shows how to register a chook hook with a process before
launching it and then call the hook once the subprocess
has been launched.
