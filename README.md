# shook

`chook` (short for Child Hook) is a Rust crate that allows
you to register a `fn<ArgT: Deserialize, RetT: Serialize>(arg: ArgT) -> RetT`
an inject it into a child process via `LD_PRELOAD`.

## But why tho?

This allows you to run code in the address space of your child process,
which lets you do some things that you can't do otherwise. If you
don't control the implementation of the child process, this can
let you do such things despite not being able to put any custom
code directly into the child process.

In particular, this is used by [shpool](https://github.com/shell-pool/shpool)
to update environment variables in child shells.

## Under the hood

Your hook routine will get packaged up in a `.so` file with an
`_init` routine that listens for a few magic env vars that tell
it

1. Where to listen on a unix domain socket for incoming calls
2. The PID that ought to be allowed to make calls (this is restricted
   for security purposes).

It will then create a unix domain socket and start listening for
RPC calls, which it will then use to call your registered hook.

When you create a subprocess, you'll be able to register a `chook`
with the process before launch, and subsequently place calls to
the hook via the same handle that you used for registration.
