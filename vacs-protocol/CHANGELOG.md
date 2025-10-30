# Changelog

## 0.1.0 (2025-10-30)


### Features

* add call error signaling message ([f66fb8b](https://github.com/MorpheusXAUT/vacs/commit/f66fb8bf1e12d04098c36af3c6b82047c6eada20))
* add protocol version to websocket login ([e778af9](https://github.com/MorpheusXAUT/vacs/commit/e778af94f3c50f807713a41b4c2553a79f82e8d6))
* change auth code exchange endpoint to POST ([fe4eb29](https://github.com/MorpheusXAUT/vacs/commit/fe4eb2978aeb4297250a4af7b1da3100748b9728))
* implement VATSIM OAuth using axum-login ([1d5f2ac](https://github.com/MorpheusXAUT/vacs/commit/1d5f2accd7d60267d3bfc3142bf02ed0c4dc0daf))
* **vacs-client:** add call error message to info grid ([8cf6bb6](https://github.com/MorpheusXAUT/vacs/commit/8cf6bb60b96187d93e1203291b4a18266680b930)), closes [#9](https://github.com/MorpheusXAUT/vacs/issues/9)
* **vacs-client:** add call queue and da key functionality to ui ([9a62fa3](https://github.com/MorpheusXAUT/vacs/commit/9a62fa3d9095c0f12d1458bda9168b6a3a0c5a40))
* **vacs-client:** add config setting for update release channel ([fab181f](https://github.com/MorpheusXAUT/vacs/commit/fab181f58bb5dbe32ea1513bb77ce76a8878f42b))
* **vacs-client:** implement logout ([c5d63c9](https://github.com/MorpheusXAUT/vacs/commit/c5d63c997d0bb751ac195ec7be3e495d5884bce2))
* **vacs-client:** wip: start implementing signaling connection ([44939ea](https://github.com/MorpheusXAUT/vacs/commit/44939ea530fc85683bb97bb8f2986d4cb0cecfe4))
* **vacs-protocol:** add CallInvite and CallAccept messages ([299816f](https://github.com/MorpheusXAUT/vacs/commit/299816f484aba3ded4459ec9804e533fc3e678aa))
* **vacs-protocol:** add ClientInfo message ([3af96a3](https://github.com/MorpheusXAUT/vacs/commit/3af96a3bb0f5e9f1764c80306668f085b2597f69))
* **vacs-protocol:** add Disconnected websocket message ([d82dceb](https://github.com/MorpheusXAUT/vacs/commit/d82dceb02eff8ca30e69fc06fae075b2c03b040b))
* **vacs-protocol:** add release and release channel types ([4c8ed01](https://github.com/MorpheusXAUT/vacs/commit/4c8ed018ee34c918d560d33d3d665ff5487891b7))
* **vacs-server:** add auth endpoint returning logged in user's info ([96d9724](https://github.com/MorpheusXAUT/vacs/commit/96d972478221c8a1bf555c60504f6d26ea285177))


### Bug Fixes

* fix tests after login refactor ([8d2c2d6](https://github.com/MorpheusXAUT/vacs/commit/8d2c2d626c75acf15dd6dc771315b3816cf209fe))
* fix tests after signaling message serialization changes ([b9eed16](https://github.com/MorpheusXAUT/vacs/commit/b9eed163250fda5764401ad829f6911b036e406c))
* **vacs-protocol:** fix hard-coded protocol version in test ([5f0be7e](https://github.com/MorpheusXAUT/vacs/commit/5f0be7e0c4f9dba69aaf034378ce217fbf597ded))
* **vacs-signaling:** remove client id from login tests ([3e4a6c4](https://github.com/MorpheusXAUT/vacs/commit/3e4a6c4581d54357ccff50576f56ce8f1b6aff34))
