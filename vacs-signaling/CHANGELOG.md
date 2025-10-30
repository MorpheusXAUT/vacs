# Changelog

## 0.1.0 (2025-10-30)


### Features

* add protocol version to websocket login ([e778af9](https://github.com/MorpheusXAUT/vacs/commit/e778af94f3c50f807713a41b4c2553a79f82e8d6))
* add SignalingClient status and logout functionality ([6842b92](https://github.com/MorpheusXAUT/vacs/commit/6842b924505987b67437294c3a2a8b5109cfeaf0))
* implement automatic reconnecting in SignalingClient ([0e71616](https://github.com/MorpheusXAUT/vacs/commit/0e716163e766064e43069474f6024550d564c3b5)), closes [#3](https://github.com/MorpheusXAUT/vacs/issues/3)
* implement new login flow returning client info ([7b6ad3e](https://github.com/MorpheusXAUT/vacs/commit/7b6ad3e7a4c14a70cdde9df3aab9a206eb95925e))
* implement VATSIM OAuth using axum-login ([1d5f2ac](https://github.com/MorpheusXAUT/vacs/commit/1d5f2accd7d60267d3bfc3142bf02ed0c4dc0daf))
* make signaling disconnect idempotent ([11f1fad](https://github.com/MorpheusXAUT/vacs/commit/11f1fadde25678f56cc21656286fd72f12380fe2))
* **vacs-client:** add signaling disconnect and display name placeholder ([a6360a3](https://github.com/MorpheusXAUT/vacs/commit/a6360a3b6be23270e6aeeec821d2d69541807639))
* **vacs-client:** emit error if signaling connection is disconnected from server side ([8622f92](https://github.com/MorpheusXAUT/vacs/commit/8622f927b0d7edd7a3fb8ef331f308be820928ed))
* **vacs-client:** wip: start implementing signaling connection ([44939ea](https://github.com/MorpheusXAUT/vacs/commit/44939ea530fc85683bb97bb8f2986d4cb0cecfe4))
* **vacs-signaling:** add additional check for sleep detection ([6c59959](https://github.com/MorpheusXAUT/vacs/commit/6c599598cdb9a189446135f28dce331ee077e444))
* **vacs-signaling:** add automatic client-side ping to detect disconnects ([769bc26](https://github.com/MorpheusXAUT/vacs/commit/769bc2649742e8f32e3d01ddb50393d4563451e5)), closes [#15](https://github.com/MorpheusXAUT/vacs/issues/15)
* **vacs-signaling:** add circuit breaker preventing reconnect loop ([c59be9e](https://github.com/MorpheusXAUT/vacs/commit/c59be9e7666673ff1601efedcdf1abc755096cd8)), closes [#65](https://github.com/MorpheusXAUT/vacs/issues/65)
* **vacs-signaling:** add disconnect reason ([fa04e66](https://github.com/MorpheusXAUT/vacs/commit/fa04e66207c6dfa3f58551c67645453a163aaeb5))
* **vacs-signaling:** automatically reply to Ping messages with Pong ([26a8f9a](https://github.com/MorpheusXAUT/vacs/commit/26a8f9a903745483daa92fe3344f8886efccadd6))
* wip: add auto reconnect to client signaling connection ([7c24819](https://github.com/MorpheusXAUT/vacs/commit/7c24819032a02d03893d518b732fb74aa1b6a964))


### Bug Fixes

* add mock data feed to fix tests ([d6bb75b](https://github.com/MorpheusXAUT/vacs/commit/d6bb75bed19fb52a27b6f5b883c8d6b159affddd))
* continue fixing client tests ([8bf41b9](https://github.com/MorpheusXAUT/vacs/commit/8bf41b9b5c93b95f064315e4b4511b2e169ad632))
* fix tests after login refactor ([8d2c2d6](https://github.com/MorpheusXAUT/vacs/commit/8d2c2d626c75acf15dd6dc771315b3816cf209fe))
* fix tests after signaling client refactor ([9e30d90](https://github.com/MorpheusXAUT/vacs/commit/9e30d900fca9e671147ba74a4a59add74e3bf0b6))
* **vacs-client:** fix async runtime handling ([21ad7bd](https://github.com/MorpheusXAUT/vacs/commit/21ad7bd70ef1cdf4541876a4252443ab09ae3cd5))
* **vacs-client:** pretty print signaling disconnected frontend error ([cc36d54](https://github.com/MorpheusXAUT/vacs/commit/cc36d54d88d8aaa2078fabdc4bba5db91bba8a3f))
* **vacs-signaling:** fix client tests ([e506ace](https://github.com/MorpheusXAUT/vacs/commit/e506ace5abff66b650a3f515d1a57e9339bec6b8))
* **vacs-signaling:** fix login test ([543392f](https://github.com/MorpheusXAUT/vacs/commit/543392f5df2ea216bb3c7daa609457afa8f6c568))
* **vacs-signaling:** remove client id from login tests ([3e4a6c4](https://github.com/MorpheusXAUT/vacs/commit/3e4a6c4581d54357ccff50576f56ce8f1b6aff34))
* **vacs-signaling:** use OnceCell to prevent send_tx propagation issues ([2380ba4](https://github.com/MorpheusXAUT/vacs/commit/2380ba4541617f6edf6b027cd2414d6d8d875e50))
* **vacs-signaling:** wip: fix client tests ([eb992e5](https://github.com/MorpheusXAUT/vacs/commit/eb992e50a9944dba9124535f5f0707af3aa48a90))
