Command permissions should work as described in [How to permissions.md](How%20to%20permissions.md).

Additionally, commands should be configurable in the following aspects:

- Aliases (and the main name)
- Auto delete input / output
- Reply or not
- Cooldown (max n uses within x duration)

All these should be easily bulk-editable. The internal command system should be designed such that adding support for slash commands (coming at some point in Fluxer) is easy without having to rewrite the whole thing, and especially not every single command.