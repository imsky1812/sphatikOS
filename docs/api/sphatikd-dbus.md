# sphatikd D-Bus API (draft v0.1)

`sphatikd` exposes the system services on the **system bus** under the well-known name `org.sphatik.Daemon`, object path `/org/sphatik/Daemon`. Apps reach these interfaces only through their sandbox's filtered D-Bus proxy; each method lists who may call it.

Types use D-Bus signature notation: `s` string, `b` bool, `u` uint32, `i` int32, `t` uint64, `d` double, `a{sv}` dictionary of variants, `as` array of strings.

## org.sphatik.Settings

| Member | Kind | Signature | Caller | Description |
| --- | --- | --- | --- | --- |
| `Get(key)` | method | `s → v` | shell, apps (own namespace) | Read one setting, for example `display.glass_tier` |
| `Set(key, value)` | method | `sv → ()` | shell; apps for keys under `app.<app-id>.*` | Write a setting; validated against the schema |
| `List(prefix)` | method | `s → a{sv}` | shell | All settings under a prefix |
| `Changed(key, value)` | signal | `sv` | | Emitted after every successful write |

Key schema (excerpt): `display.glass_tier` (`"liquid"`, `"balanced"`, `"low"`), `display.dark_mode` (`"auto"`, `"on"`, `"off"`), `lock.clock` (a{sv}: font, weight, colour, finish), `home.grid` (as), `home.widgets` (as), `wallpaper.id` (s).

## org.sphatik.Permissions

| Member | Kind | Signature | Caller | Description |
| --- | --- | --- | --- | --- |
| `Check(app_id, permission)` | method | `ss → s` | portals | Returns `"granted"`, `"denied"`, `"ask"` or `"granted-once"` |
| `Request(app_id, permission, reason)` | method | `sss → s` | portals | Shows the system prompt; returns the user's choice |
| `Revoke(app_id, permission)` | method | `ss → ()` | Settings app | Revoke a grant |
| `AccessLog(app_id, since)` | method | `st → a(sst)` | Settings app | `(permission, timestamp, detail)` entries for the privacy dashboard |
| `InUse(permission, app_id, active)` | signal | `ssb` | | Drives the camera, microphone and location dots |

Permissions: `camera`, `microphone`, `location.precise`, `location.approximate`, `contacts`, `calendar`, `photos`, `files`, `notifications`, `network`, `background`, `bluetooth`, `nearby`.

## org.sphatik.Notifications

| Member | Kind | Signature | Caller | Description |
| --- | --- | --- | --- | --- |
| `Post(app_id, channel, notification)` | method | `ssa{sv} → u` | apps via portal, Android bridge | Returns a notification id. Keys: `title`, `body`, `icon`, `actions` (as), `reply` (b), `importance` (`urgent`, `normal`, `quiet`, `summary`) |
| `Update(id, notification)` | method | `ua{sv} → ()` | posting app | Update in place |
| `Withdraw(id)` | method | `u → ()` | posting app | Remove |
| `DeclareChannel(app_id, channel, meta)` | method | `ssa{sv} → ()` | apps | Register a channel with a default importance |
| `ActionInvoked(id, action, reply_text)` | signal | `uss` | | Sent to the posting app when the user taps an action or replies |
| `Dismissed(id, reason)` | signal | `us` | | User dismissed, snoozed, or it was summarised |

## org.sphatik.Apps

| Member | Kind | Signature | Caller | Description |
| --- | --- | --- | --- | --- |
| `List()` | method | `→ aa{sv}` | shell | Installed apps: `id`, `name`, `icon`, `kind` (`native`, `android`), `categories` |
| `Launch(app_id, intent)` | method | `sa{sv} → ()` | shell, portals | Start an app, optionally with an intent |
| `Install(path)` | method | `s → s` | Store, file manager (with user confirmation) | Verify signature and install an `.spk`; returns the app id |
| `Uninstall(app_id)` | method | `s → ()` | Settings, shell | Remove an app and its data |
| `Installed(app_id)`, `Removed(app_id)` | signals | `s` | | Keep Home and the App Library in sync |

## org.sphatik.LiveActivities

| Member | Kind | Signature | Caller | Description |
| --- | --- | --- | --- | --- |
| `Start(app_id, template, state)` | method | `ssa{sv} → u` | apps (own activities), system services | Templates: `compact`, `progress`, `media`, `custom`; `kind` in state sets priority: `call`, `navigation`, `timer`, `media`, `download` |
| `Update(id, state)` | method | `ua{sv} → ()` | owning app | Limited to once per second |
| `End(id)` | method | `u → ()` | owning app | Removes it from the Halo |
| `Changed()` | signal | `()` | | Shell re-reads the active set |

## org.sphatik.Power

| Member | Kind | Signature | Caller | Description |
| --- | --- | --- | --- | --- |
| `Battery` | property | `a{sv}` | anyone | `percent`, `charging`, `time_to_empty`, `health` |
| `Thermal` | property | `s` | anyone | `normal`, `warm`, `hot`, `critical` |
| `SetSaver(on)` | method | `b → ()` | Control Center, Settings | Battery saver |
| `Inhibit(app_id, what, reason)` | method | `sss → h` | apps with `background` permission | Returns a file descriptor; suspend is blocked while it stays open |

## Error names

`org.sphatik.Error.PermissionDenied`, `.NotFound`, `.InvalidArgument`, `.RateLimited`, `.SignatureInvalid`, `.Busy`.

## Versioning

Interfaces carry a `Version` property (`u`). Additive changes bump the minor version; breaking changes create a new interface name (`org.sphatik.Notifications2`).
