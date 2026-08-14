# Release automation

Two things happen on their own once a `vacs-client` release is built, both as jobs at the end of
[release-client](../.github/workflows/release-client.yml), in this order:

1. **The server's release catalog is reloaded**, so the updater offers the new version immediately.
2. **The release is announced in Discord**, with a message assembled from the What's New page.

Server releases do neither.

---

## Reloading the release catalog

The server's update checker reads releases from the GitHub API and caches them for four hours by
default (`release_cache_ttl`). Without a nudge, a version that has just shipped stays invisible to
the updater for up to that long.

The `reload-releases` job calls `POST /admin/releases/reload` on the server, which repopulates the
catalog and prefetches the updater signatures for the newest release of each channel. It runs for
prereleases too, since the `rc` channel serves those.

It then **verifies the result** rather than trusting the `200`, by asking `GET /version/update` what
a client on `0.0.1` would be offered, for every platform the build ships:

```
windows/x86_64/nsis
linux/x86_64/deb
linux/x86_64/rpm
darwin/x86_64/app
darwin/aarch64/app
```

Every one of them must come back with the new version. Reload and check are one retry loop, not two
steps, because the GitHub API can still be reporting the release without its assets when the first
reload lands, and a release with no assets is dropped from the catalog entirely; only another reload
fixes that.

> [!NOTE]
> Keep the `bundles` list in step with what the build actually produces. It is deliberately strict:
> a platform silently dropping out of the release is the failure this catches. AppImage is absent
> because Tauri no longer bundles one, and Windows is `nsis`, not `msi`. Before this check existed,
> asking the live server for `msi` returned **0.3.0**, the last release that shipped one.

### Servers and configuration

Both deployments are reloaded, as a matrix over the `dev` and `production` environments, taking the
server URL from the environment-scoped `VACS_SERVER_URL` variable:

| Environment | Server | OIDC subject the server must allow |
| --- | --- | --- |
| `dev` | `https://dev.vacs.network` | `repo:vacs-project/vacs:environment:dev` |
| `production` | `https://vacs.network` | `repo:vacs-project/vacs:environment:production` |

The dev leg runs with `continue-on-error`, so a dev server that is down cannot hold up a production
release or block the announcement.

Authentication is a GitHub Actions OIDC token, the same mechanism the `vacs-data` repository uses
for `POST /admin/dataset/reload`. The two endpoints deliberately use **separate** allowed subjects,
so that letting one repository reload the dataset never also lets it reload the release catalog.

Each server's `config.toml` allows only its own environment:

```toml
[admin]
oidc_audience = "https://vacs.network"
oidc_allowed_sub_releases = "repo:vacs-project/vacs:environment:production"  # dev uses :environment:dev
```

Leave `oidc_allowed_sub_releases` unset and the endpoint answers `404` to everyone, including a
valid token. The subject comes from the job's GitHub Environment, which is why `reload-releases`
declares one; without it the subject would be the git ref and would change with every release.

Repository configuration: `dev` and `production` environments, `VACS_SERVER_URL` scoped to each,
and `VACS_OIDC_AUDIENCE`.

If the production reload fails, the announcement does not go out either, because announcing an
update the server will not serve for another four hours is worse than announcing late. Fix the cause
and announce by hand.

---

## Announcing in Discord

The message is assembled by [`scripts/discord-announce.py`](../scripts/discord-announce.py) and
posted by the vacs Discord application through the
[announce-release](../.github/workflows/announce-release.yml) workflow. It is gated on the release
being neither a draft nor a prerelease, and it runs only after the installers are uploaded, the
catalog is reloaded and the Homebrew cask is bumped.

> [!IMPORTANT]
> Do not move this to a `release: published` trigger. release-please creates the
> `vacs-client-vX.Y.Z` release and tag the moment the release PR merges, which is long before
> anything has been built. That event fires while the release page is still empty.

### Where the text comes from

Nothing about the announcement is written by the workflow. It is taken from the What's New page of
the documentation site, `docs/whats-new.mdx` in the `vacs-project.github.io` repository, which
already has a section per release:

```md
## v2.6.0

[v2.6.0](https://github.com/vacs-project/vacs/releases/tag/vacs-client-v2.6.0) lets you use joystick
and gamepad buttons for every key binding, ...

### Joystick and gamepad buttons as key bindings
```

- The **lead paragraph** becomes the body of the announcement. The `[v2.6.0](...)` link at the front is dropped, since the release URL is already on its own line, and any docs-relative links are rewritten to absolute ones.
- The **`###` headings** become the highlights list, minus generic ones like "Bug fixes" and "Other improvements". A release left with fewer than two headings is announced as plain prose, which is what a patch release should look like.
- The **release-please changelog is not repeated.** Discord unfurls the release URL and renders the release body in the embed, so it is in the message already.

Writing the What's New section is therefore the only manual step, and it is required by the
documentation rules anyway.

---

### Guards

Two guards stop the announcement going out early, on the automatic and the manual path alike:

1. The GitHub release must exist, be published, and carry at least one installer asset.
2. `https://docs.vacs.network/whats-new` must actually show the version's section. The job waits up to ten minutes for the docs deploy and then fails rather than linking to a page that does not mention the release.

A failure in either guard leaves the release itself untouched. Fix the cause and announce by hand,
below.

---

### Announcing by hand

Run the **Announce • vacs-client** workflow from the Actions tab. Inputs:

| Input | Meaning |
| --- | --- |
| `version` | `X.Y.Z`, without the `vacs-client-v` prefix |
| `blurb` | replaces the lead paragraph, for when the docs do not say what you want to say |
| `dry_run` | on by default; renders the message into the job summary and posts nothing |
| `publish` | also presses Publish, if the channel is an Announcement channel |

Leave `dry_run` on for the first run, read the job summary, then run it again with `dry_run` off.

To preview locally against a docs checkout, without touching the network:

```bash
python3 scripts/discord-announce.py --version 2.6.0 --dry-run \
    --whats-new ../vacs-project.github.io/docs/whats-new.mdx
```

---

### Discord application setup

The announcement is posted by a Discord application, not a webhook, so that it can also publish
in an Announcement channel.

Repository configuration:

| Name | Kind | Where it comes from |
| --- | --- | --- |
| `DISCORD_BOT_TOKEN` | secret | Developer Portal, the app, **Bot** tab, Reset Token. Shown once |
| `DISCORD_CHANNEL_ID` | variable | with Developer Mode enabled, right-click the channel, Copy Channel ID |
| `DISCORD_RELEASE_ROLE_ID` | variable | Server Settings, Roles, right-click the role, Copy Role ID |

The app must be invited to the server. Take the Application ID from General Information and open:

```
https://discord.com/oauth2/authorize?client_id=YOUR_APP_ID&scope=bot&permissions=18432
```

`18432` is Send Messages (2048) plus Embed Links (16384). The same can be clicked together under
OAuth2, URL Generator.

| Permission | Why |
| --- | --- |
| Send Messages | posting at all |
| Embed Links | without it the message posts as bare URLs and the GitHub and docs embeds do not render |
| Mention @everyone, @here and All Roles (131072) | only if the announcement role is not marked mentionable. Ticking "Allow anyone to @mention this role" in the role settings is the tidier option. Use `permissions=149504` to include it |

No gateway intents are needed; the app never connects, it makes one HTTPS request per release.
Check the announcements channel for permission overrides too, since a channel override beats the
server-wide grant.

The role ping is sent with `allowed_mentions.parse` empty and the single role id listed, so no
mention can fire other than that role, whatever the release notes happen to contain.
