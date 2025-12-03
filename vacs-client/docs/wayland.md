# Wayland Support

`vacs` supports running on Wayland compositors, but due to the stricter security model of Wayland compared to X11, there are some differences in functionality and configuration.

## Keybinds (Global Shortcuts)

`vacs` supports global keybinds on Wayland compositors that implement the [XDG Desktop Portal's](https://flatpak.github.io/xdg-desktop-portal/) [Global Shortcuts](https://flatpak.github.io/xdg-desktop-portal/#gdbus-org.freedesktop.portal.GlobalShortcuts) interface. This includes most modern compositors like KDE Plasma (via `xdg-desktop-portal-kde`), GNOME (via `xdg-desktop-portal-gnome`), and Hyprland (via `xdg-desktop-portal-hyprland`).

### Configuration

Unlike on Windows or macOS, applications on Wayland cannot arbitrarily listen to keyboard input when they are not focused. Instead, they must request permission from the compositor to register global shortcuts.

When you launch `vacs` for the first time, it will attempt to register its shortcuts with the system. Depending on your desktop environment, you may see a system dialog asking you to grant permission and configure the key combinations.

1.  **Launch `vacs`**.
2.  If prompted by your system, **allow** the application to register shortcuts.
3.  A configuration window provided by your desktop environment should appear.
4.  Bind the keys you wish to use for **Push-to-Talk**, **Push-to-Mute**, **Radio Integration** etc.
5.  If you dismiss the dialog or want to change bindings later, you can usually find them in your system settings under "Global Shortcuts" or "Applications". You can also click the externally bound key in the `vacs` settings to try to open the appropriate settings page for your desktop environment.

### Limitations

Due to the security model of Wayland and the design of the Global Shortcuts portal, there are significant limitations compared to X11, Windows, or macOS:

-   **No Modifiers**: You cannot bind standalone modifier keys (e.g., just `Ctrl`, `Alt`, or `Shift`) as a shortcut. You must use a non-modifier key, or a combination of modifier keys and a non-modifier key (e.g., `Ctrl+Alt+P`).
-   **No Regular Keys**: You generally cannot use regular alphanumeric keys (like `A`, `1`, `Space`) as global shortcuts, as these are reserved for typing in the focused application.
-   **Compositor Dependent**: The user interface for configuring shortcuts is entirely provided by your desktop environment. `vacs` has no control over how this looks or behaves. Additionally, we cannot reliably re-trigger the shortcut configuration UI if it was dismissed as compositors might only show it once per session. If you dismiss the dialog, you will need to reconfigure your shortcuts manually in your system settings.

### Troubleshooting

-   **"Portal unavailable"**: Ensure you have `xdg-desktop-portal` and a backend implementation for your desktop environment installed (e.g., `xdg-desktop-portal-kde`, `xdg-desktop-portal-gnome`, `xdg-desktop-portal-hyprland`).
-   **Shortcuts not working**: Check your system settings to ensure the shortcuts are still granted to `vacs`. Some compositors may revoke permissions if the application is updated or changed.

## Radio Integration (Key Emulation)

`vacs` supports integrating with other radio clients (like Audio For VATSIM or TrackAudio) by emulating key presses when you transmit. This allows you to use a single PTT key for both `vacs` and your radar client's radio.

**However, this feature is currently NOT fully supported on Wayland.**

Wayland does not provide a standard, secure way for applications to inject input events (like key presses) into other applications. While there are some compositor-specific protocols (like `wlr-virtual-input`), there is no cross-desktop standard that `vacs` can rely on.

As a result, the **Radio Integration** feature will not work for integrations relying on external key presses on Wayland. You will need to use a radio integration providing direct support (e.g., via an API) or configure your radio client and `vacs` separately, for instance using the "Push-to-Mute" transmit mode.  
Note that since `vacs` uses the Global Shortcuts portal, it can share the same physical key as another application if that application also listens globally (or if the compositor allows it). However, `vacs` cannot _trigger_ the other application's PTT.
