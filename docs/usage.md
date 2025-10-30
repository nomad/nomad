# Usage

There are two main ways to interact with Nomad:

- through the Neovim command line, using the `:Mad` command;
- through the Lua API, by requiring the `"nomad"` module;

For example, you can start a new collaborative editing session with either:

```vim
:Mad collab start
```

or

```lua
require("nomad").collab.start()
```

In general, any command following the `:Mad <module> <action>` pattern has a
corresponding Lua function under `require("nomad").<module>.<action>`. Use the
command line for quick, interactive operations, and the Lua API when writing
scripts or setting up keybindings.

For brevity, the rest of this document will only show commands in the `:Mad
<module> <action>` format.

## `:Mad auth login`

This command opens a new browser window, prompting you to authenticate with
Nomad using your GitHub credentials. Logging in is necessary to start and join
collaborative editing sessions.

After a successfull login, the corresponding credentials will be persisted on
your machine using the system credential store.

## `:Mad auth logout`

This command removes any credentials stored by `:Mad auth login` from the
credential store.

## `:Mad collab start`

This command starts a new collaborative editing session. Before you run it,
make sure to place your cursor anywhere in any file that's under the root of
the project you want to start collaborating on.

When you run this command, Nomad will index all files under the project root
and ask the [collab server][collab-server] to start a new session. If the
request succeeds, a new, unique session ID will be returned.

The session ID should be treated as a secret, as it allows anyone to join the
session and receive a copy of the project.

NOTE: the collab server never stores any of your files, neither during the
session nor after it completes. It simply acts as a one-to-many network channel
that forwards every peer's events to every other peer.

By default, Nomad will reach out to the server running at `collab.nomad.foo`.
Like the rest of our code, the server is open source and MIT licensed, so you
can take it and run it on your own infrastructure if you don't trust us.

## `:Mad collab join <session_id>`

This command lets you join an existing collaborative editing session. When you
run it, it ask the collab server to add you to the session with the given ID.
If that succeeds, a copy of the project will be requested from one of the other
peers currently in the session.

Obtaining the initial copy can take a while, depending on the size of the
project, the upload speed of the project's sender, and your download speed.

Once the project has been received, it will be written to disk under
`$XDG_DATA_HOME/nvim/nomad/collab/remote-projects/<project_name>`, and you'll
be prompted to jump to the position of another peer that's already in it (by
default, the host).

From that point on, you can start editing all files in the project as you
normally would! Every time you make an edit, move your cursor, select some
text, or save a buffer, that event will be sent to the other peers in the
session, creating the illusion of a shared workspace while everyone
independently works on their own copy.

## `:Mad collab copy-id`

This command copies the session ID of the collaborative session you're
currently in to your clipboard.

## `:Mad collab jump <github_handle>`

This command lets you "jump" to the current position of the peer with the given
GitHub handle, wherever they currently are in the project you're collaborating
on. This will create a new buffer if necessary.

## `:Mad collab leave`

This command lets you leave the collaborative editing session you're currently
in. Note that if the host of the session leaves, the session will end for all
remaining peers currently in it (this limitation may be removed in the future).

## `:Mad collab pause`

This command causes Nomad to stop applying the remote events being received
from the other peers. While a session is paused, all incoming events are
buffered in memory until the session is resumed (see the following command).

This is mostly useful when debugging the CRDT machinery that keeps the project
state synchronized between peers. For example, several peers could pause the
session, apply different edits that would usually cause conflicts when using
more traditional version control tools (e.g. Git), and resume the session.

When resumed, the final state should converge across all peers, regardless of
what operations each peer applied while paused or the order in which those
operations are received.

It's also a cool party trick.

## `:Mad collab resume`

This command resumes a session previously paused by `:Mad collab pause`. All
events that had been buffered while the session was paused will be applied at
once.

[collab-server]: https://github.com/nomad/collab-server
