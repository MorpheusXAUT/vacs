#!/usr/bin/env python3
"""Render and post the Discord release announcement for a vacs-client release.

The announcement prose is not written here. It is lifted from the What's New page of
the documentation site (vacs-project.github.io, docs/whats-new.mdx), which already
carries a hand-written lead paragraph and a set of section headings for every release:

    ## v2.6.0

    [v2.6.0](https://github.com/.../vacs-client-v2.6.0) lets you use joystick and
    gamepad buttons for every key binding, ...

    ### Joystick and gamepad buttons as key bindings

The lead paragraph becomes the body of the announcement and the section headings become
the highlights list. The release-please changelog is deliberately not included: Discord
unfurls the release URL and renders it in the embed already.

Usage:
    # preview, no network writes
    python3 scripts/discord-announce.py --version 2.6.0 --dry-run

    # preview against a local checkout of the docs repo
    python3 scripts/discord-announce.py --version 2.6.0 --dry-run \
        --whats-new ../vacs-project.github.io/docs/whats-new.mdx

    # post for real
    DISCORD_BOT_TOKEN=... DISCORD_CHANNEL_ID=... DISCORD_RELEASE_ROLE_ID=... \
        python3 scripts/discord-announce.py --version 2.6.0

Environment:
    DISCORD_BOT_TOKEN        bot token of the Discord application (required to post)
    DISCORD_CHANNEL_ID       channel to post in (required to post)
    DISCORD_RELEASE_ROLE_ID  role to ping, omit for no ping
"""

from __future__ import annotations

import argparse
import json
import os
import re
import string
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
TEMPLATE_FILE = SCRIPT_DIR / "templates" / "discord-release.md"

DOCS_SITE = "https://docs.vacs.network"
WHATS_NEW_URL = f"{DOCS_SITE}/whats-new"
WHATS_NEW_SOURCE = (
    "https://raw.githubusercontent.com/vacs-project/vacs-project.github.io"
    "/main/docs/whats-new.mdx"
)
REPO_API = "https://api.github.com/repos/vacs-project/vacs"
RELEASE_URL_PREFIX = "https://github.com/vacs-project/vacs/releases/tag"
DISCORD_API = "https://discord.com/api/v10"

USER_AGENT = "vacs-discord-announce"
MAX_MESSAGE_LENGTH = 2000

# Headings that describe nothing specific about the release and make poor highlights.
GENERIC_HEADINGS = {"bug fixes", "other improvements", "other changes"}

# A highlights list of one is not a list; render those releases as plain prose instead,
# which is how patch releases were announced by hand.
MIN_HIGHLIGHTS = 2

INSTALLER_SUFFIXES = (".msi", ".exe", ".dmg", ".AppImage", ".deb", ".rpm")

VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?$")


class AnnounceError(Exception):
    """A condition that must stop the announcement rather than post something wrong."""


# --------------------------------------------------------------------------------------
# Fetching
# --------------------------------------------------------------------------------------


def _get(url: str, *, headers: dict[str, str] | None = None) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, **(headers or {})})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read()


def fetch_whats_new(source: str) -> str:
    """Read whats-new.mdx from a local path or over https."""
    if source.startswith(("http://", "https://")):
        # The vacs repos are public; sending credentials here would only break the request.
        try:
            return _get(source).decode("utf-8")
        except urllib.error.URLError as err:
            raise AnnounceError(f"could not fetch {source}: {err}") from err

    path = Path(source)
    if not path.is_file():
        raise AnnounceError(f"no such file: {path}")
    return path.read_text(encoding="utf-8")


# --------------------------------------------------------------------------------------
# Extraction
# --------------------------------------------------------------------------------------


def extract_section(text: str, version: str) -> tuple[str, list[str]]:
    """Return the lead paragraph and the highlight headings for `## vX.Y.Z`."""
    lines = text.splitlines()
    heading = f"## v{version}"

    try:
        start = next(i for i, line in enumerate(lines) if line.strip() == heading)
    except StopIteration:
        raise AnnounceError(
            f"whats-new.mdx has no '{heading}' section. Write the release notes first, "
            f"or pass --blurb to announce without them."
        ) from None

    end = len(lines)
    for i in range(start + 1, len(lines)):
        if lines[i].startswith("## "):
            end = i
            break

    section = lines[start + 1 : end]
    return _lead_paragraph(section, heading), _highlights(section)


def _lead_paragraph(section: list[str], heading: str) -> str:
    """The first prose paragraph of the section, joined into a single line."""
    paragraph: list[str] = []
    for line in section:
        stripped = line.strip()
        if not stripped:
            if paragraph:
                break
            continue
        # Skip imagery and admonition markers that may precede the prose.
        if stripped.startswith(("<", ":::", "|", "```")):
            continue
        paragraph.append(stripped)

    if not paragraph:
        raise AnnounceError(f"'{heading}' has no lead paragraph to announce")
    return " ".join(paragraph)


def _highlights(section: list[str]) -> list[str]:
    """The `###` headings of the section, minus the ones that say nothing."""
    headings: list[str] = []
    in_fence = False
    for line in section:
        if line.startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence or not line.startswith("### "):
            continue
        title = line[4:].strip()
        if title.lower() not in GENERIC_HEADINGS:
            headings.append(title)
    return headings


def mdx_to_discord(markdown: str, version: str) -> str:
    """Turn a whats-new paragraph into something Discord renders correctly."""
    # The version link duplicates the release URL that already sits on its own line.
    text = re.sub(
        rf"^\[v{re.escape(version)}\]\([^)]*\)",
        f"v{version}",
        markdown,
    )
    # Docs-relative links only resolve on the docs site.
    text = re.sub(r"\]\((/[^)]*)\)", rf"]({DOCS_SITE}\1)", text)
    return text


def anchor_slug(version: str) -> str:
    """Docusaurus heading id for `## vX.Y.Z`, e.g. v2.6.0 -> v260."""
    slug = f"v{version}".lower()
    slug = re.sub(r"[^a-z0-9\s-]", "", slug)
    return re.sub(r"\s+", "-", slug).strip("-")


# --------------------------------------------------------------------------------------
# Pre-flight guards
# --------------------------------------------------------------------------------------


def check_release(version: str) -> None:
    """Refuse to announce a release that has no installers on it yet.

    release-please creates the GitHub release when the release PR merges, long before
    release-client.yml has built and uploaded anything. Linking people to that empty
    release is the failure this guards against.
    """
    tag = f"vacs-client-v{version}"
    try:
        payload = json.loads(_get(f"{REPO_API}/releases/tags/{tag}"))
    except urllib.error.HTTPError as err:
        if err.code == 404:
            raise AnnounceError(f"no GitHub release tagged {tag}") from err
        raise AnnounceError(f"could not read release {tag}: {err}") from err
    except urllib.error.URLError as err:
        raise AnnounceError(f"could not read release {tag}: {err}") from err

    if payload.get("draft"):
        raise AnnounceError(f"release {tag} is still a draft")
    if payload.get("prerelease"):
        raise AnnounceError(f"release {tag} is a prerelease and is not announced")

    installers = [
        asset["name"]
        for asset in payload.get("assets", [])
        if asset.get("name", "").endswith(INSTALLER_SUFFIXES)
    ]
    if not installers:
        raise AnnounceError(
            f"release {tag} carries no installers yet. The build has not finished, "
            f"and the announcement would point at an empty release page."
        )
    print(f"release {tag}: {len(installers)} installers present", file=sys.stderr)


def wait_for_docs(version: str, timeout: int) -> None:
    """Block until the What's New page actually shows this version's section."""
    anchor = anchor_slug(version)
    pattern = re.compile(rf'id="?{re.escape(anchor)}"?[\s>]')
    deadline = time.monotonic() + timeout

    while True:
        try:
            page = _get(WHATS_NEW_URL).decode("utf-8", errors="replace")
            if pattern.search(page):
                print(f"docs live: {WHATS_NEW_URL}#{anchor}", file=sys.stderr)
                return
        except urllib.error.URLError as err:
            print(f"docs check failed, retrying: {err}", file=sys.stderr)

        if time.monotonic() >= deadline:
            raise AnnounceError(
                f"{WHATS_NEW_URL} still has no #{anchor} section after {timeout}s. "
                f"The docs deploy has not landed; announcing now would link to a page "
                f"that does not mention this release."
            )
        print(f"waiting for docs deploy to publish #{anchor} ...", file=sys.stderr)
        time.sleep(30)


# --------------------------------------------------------------------------------------
# Rendering
# --------------------------------------------------------------------------------------


def render(
    version: str, lead: str, highlights: list[str], role_id: str | None, whats_new_url: str
) -> str:
    template = string.Template(TEMPLATE_FILE.read_text(encoding="utf-8"))

    if len(highlights) >= MIN_HIGHLIGHTS:
        bullets = "\n".join(f"- {title}" for title in highlights)
        highlight_block = f"\n**Highlights**\n{bullets}\n"
    else:
        highlight_block = ""

    message = template.substitute(
        role_ping=f"<@&{role_id}>" if role_id else "",
        version=version,
        release_url=f"{RELEASE_URL_PREFIX}/vacs-client-v{version}",
        lead=lead,
        highlights=highlight_block,
        whats_new_url=whats_new_url,
    ).lstrip()

    if len(message) > MAX_MESSAGE_LENGTH:
        raise AnnounceError(
            f"message is {len(message)} characters, Discord allows {MAX_MESSAGE_LENGTH}. "
            f"Shorten the lead paragraph in whats-new.mdx or pass --blurb."
        )
    return message


# --------------------------------------------------------------------------------------
# Discord
# --------------------------------------------------------------------------------------


def _discord(path: str, token: str, payload: dict | None = None) -> dict:
    request = urllib.request.Request(
        f"{DISCORD_API}{path}",
        data=json.dumps(payload).encode("utf-8") if payload is not None else b"",
        headers={
            "Authorization": f"Bot {token}",
            "Content-Type": "application/json",
            "User-Agent": USER_AGENT,
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = response.read()
    except urllib.error.HTTPError as err:
        detail = err.read().decode("utf-8", errors="replace")
        raise AnnounceError(f"Discord returned {err.code} for {path}: {detail}") from err
    except urllib.error.URLError as err:
        raise AnnounceError(f"could not reach Discord: {err}") from err

    return json.loads(body) if body else {}


def post(content: str, token: str, channel_id: str, role_id: str | None) -> str:
    """Post the announcement and return the message id.

    `parse: []` blocks every mention Discord would otherwise infer from the text, so the
    role ping below is the only one that can ever fire.
    """
    payload = {
        "content": content,
        "allowed_mentions": {"parse": [], "roles": [role_id] if role_id else []},
    }
    message = _discord(f"/channels/{channel_id}/messages", token, payload)
    return message["id"]


def crosspost(message_id: str, token: str, channel_id: str) -> None:
    """Press Publish on an Announcement channel message. Best effort."""
    try:
        _discord(f"/channels/{channel_id}/messages/{message_id}/crosspost", token)
        print("published to following servers", file=sys.stderr)
    except AnnounceError as err:
        # The announcement is already up; a failed publish is not worth failing a release.
        print(f"warning: could not publish the message: {err}", file=sys.stderr)


# --------------------------------------------------------------------------------------
# Entry point
# --------------------------------------------------------------------------------------


def write_step_summary(message: str, posted: bool) -> None:
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if not summary:
        return
    heading = "Posted to Discord" if posted else "Preview (nothing was posted)"
    with open(summary, "a", encoding="utf-8") as handle:
        handle.write(f"## {heading}\n\n```\n{message}\n```\n")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--version", required=True, help="release version, e.g. 2.6.0")
    parser.add_argument(
        "--whats-new",
        default=WHATS_NEW_SOURCE,
        help="path or URL of whats-new.mdx (default: raw file on the docs repo main branch)",
    )
    blurb = parser.add_mutually_exclusive_group()
    blurb.add_argument("--blurb", help="use this text instead of the extracted lead paragraph")
    blurb.add_argument("--blurb-file", help="read the replacement lead paragraph from a file")
    parser.add_argument("--dry-run", action="store_true", help="render only, post nothing")
    parser.add_argument("--publish", action="store_true", help="also publish in an Announcement channel")
    parser.add_argument("--skip-docs-check", action="store_true", help="do not wait for the docs deploy")
    parser.add_argument("--skip-release-check", action="store_true", help="do not verify release assets")
    parser.add_argument("--docs-timeout", type=int, default=600, help="seconds to wait for the docs deploy")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)

    try:
        version = args.version.lstrip("v")
        if not VERSION_RE.match(version):
            raise AnnounceError(f"not a version: {args.version}")

        override = None
        if args.blurb_file:
            override = Path(args.blurb_file).read_text(encoding="utf-8").strip()
        elif args.blurb:
            override = args.blurb.strip()

        documented = True
        if override is None:
            raw_lead, highlights = extract_section(fetch_whats_new(args.whats_new), version)
            lead = mdx_to_discord(raw_lead, version)
        else:
            lead = override
            # An override is also the escape hatch for a release the docs do not cover
            # yet, so a missing section here means no highlights rather than an error.
            try:
                _, highlights = extract_section(fetch_whats_new(args.whats_new), version)
            except AnnounceError as err:
                print(f"no highlights: {err}", file=sys.stderr)
                highlights = []
                documented = False

        # An undocumented release has no section anchor to link or to wait for, so the
        # What's New link points at the page top and the docs gate below is skipped.
        # Without this, the --blurb escape hatch would fail its own docs check.
        if documented:
            whats_new_url = f"{WHATS_NEW_URL}#{anchor_slug(version)}"
        else:
            whats_new_url = WHATS_NEW_URL

        role_id = os.environ.get("DISCORD_RELEASE_ROLE_ID") or None
        message = render(version, lead, highlights, role_id, whats_new_url)

        if args.dry_run:
            print(message)
            write_step_summary(message, posted=False)
            return 0

        token = os.environ.get("DISCORD_BOT_TOKEN")
        channel_id = os.environ.get("DISCORD_CHANNEL_ID")
        if not token or not channel_id:
            raise AnnounceError("DISCORD_BOT_TOKEN and DISCORD_CHANNEL_ID must both be set to post")

        if not args.skip_release_check:
            check_release(version)
        if not args.skip_docs_check and documented:
            wait_for_docs(version, args.docs_timeout)

        message_id = post(message, token, channel_id, role_id)
        print(f"posted message {message_id}", file=sys.stderr)
        if args.publish:
            crosspost(message_id, token, channel_id)
        write_step_summary(message, posted=True)
        return 0

    except AnnounceError as err:
        print(f"error: {err}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
