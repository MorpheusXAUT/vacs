# Release announcements

Every stable `vacs-client` release is announced in Discord automatically. The message is assembled
by [`scripts/discord-announce.py`](../scripts/discord-announce.py) and posted by the vacs Discord
application through the [announce-release](../.github/workflows/announce-release.yml) workflow.

Server releases are not announced.

---

## Where the text comes from

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

## When it fires

The announcement is the last job of [release-client](../.github/workflows/release-client.yml),
gated on the release being neither a draft nor a prerelease, and running only after the installers
are uploaded and the Homebrew cask is bumped.

> [!IMPORTANT]
> Do not move this to a `release: published` trigger. release-please creates the
> `vacs-client-vX.Y.Z` release and tag the moment the release PR merges, which is long before
> anything has been built. That event fires while the release page is still empty.

Two guards stop the announcement going out early, on the automatic and the manual path alike:

1. The GitHub release must exist, be published, and carry at least one installer asset.
2. `https://docs.vacs.network/whats-new` must actually show the version's section. The job waits up to ten minutes for the docs deploy and then fails rather than linking to a page that does not mention the release.

A failure in either guard leaves the release itself untouched. Fix the cause and announce by hand,
below.

---

## Announcing by hand

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

## Discord application setup

The announcement is posted by a Discord application, not a webhook, so that it can also publish
in an Announcement channel.

Repository configuration:

| Name | Kind | Where it comes from |
| --- | --- | --- |
| `DISCORD_BOT_TOKEN` | secret | Developer Portal, the app, **Bot** tab, Reset Token. Shown once |
| `DISCORD_CHANNEL_ID` | variable | with Developer Mode enabled, right-click the channel, Copy Channel ID |
| `DISCORD_RELEASE_ROLE_ID` | variable | Server Settings, Roles, right-click the role, Copy Role ID |

The app must be invited to the server (Developer Portal, OAuth2 URL Generator, scope `bot`) with:

| Permission | Why |
| --- | --- |
| Send Messages | posting at all |
| Embed Links | without it the message posts as bare URLs and the GitHub and docs embeds do not render |
| Mention @everyone, @here and All Roles | only if the announcement role is not marked mentionable. Ticking "Allow anyone to @mention this role" in the role settings is the tidier option |

No gateway intents are needed; the app never connects, it makes one HTTPS request per release.
Check the announcements channel for permission overrides too, since a channel override beats the
server-wide grant.

The role ping is sent with `allowed_mentions.parse` empty and the single role id listed, so no
mention can fire other than that role, whatever the release notes happen to contain.
