# Changelog

## [1.0.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-signaling-v0.4.0...vacs-signaling-v1.0.0) (2026-02-20)


### ⚠ BREAKING CHANGES

* implement station coverage calculations and calling ([#452](https://github.com/MorpheusXAUT/vacs/issues/452))
* overhaul UI with geo/tabbed layout and station-based calling ([#531](https://github.com/MorpheusXAUT/vacs/issues/531))

### Features

* add priority calls ([#504](https://github.com/MorpheusXAUT/vacs/issues/504)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* implement station coverage calculations and calling ([#452](https://github.com/MorpheusXAUT/vacs/issues/452)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* overhaul UI with geo/tabbed layout and station-based calling ([#531](https://github.com/MorpheusXAUT/vacs/issues/531)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** add call start and end sounds ([#505](https://github.com/MorpheusXAUT/vacs/issues/505)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** add keybind for toggling radio prio ([#500](https://github.com/MorpheusXAUT/vacs/issues/500)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** add window zoom hotkeys ([#522](https://github.com/MorpheusXAUT/vacs/issues/522)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** implement telephone directory ([#490](https://github.com/MorpheusXAUT/vacs/issues/490)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))


### Bug Fixes

* **vacs-client:** fix error while switching to exclusive audio device ([#498](https://github.com/MorpheusXAUT/vacs/issues/498)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-client:** prevent call queue from shrinking ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))
* **vacs-server:** ignore datafeed SUP connection ([#480](https://github.com/MorpheusXAUT/vacs/issues/480)) ([384131b](https://github.com/MorpheusXAUT/vacs/commit/384131bf18dbe8240602d6f4e0b226fb04effdf3))

## [0.4.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-signaling-v0.3.0...vacs-signaling-v0.4.0) (2025-11-30)


### Features

* provide TURN servers for traversing restrictive networks ([#248](https://github.com/MorpheusXAUT/vacs/issues/248)) ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))
* **vacs-client:** add profile select to mission page ([ad36dc5](https://github.com/MorpheusXAUT/vacs/commit/ad36dc55e2e42619eff9c0163e869f64910998bb))
* **vacs-client:** add station filter and aliasing ([#233](https://github.com/MorpheusXAUT/vacs/issues/233)) ([ad36dc5](https://github.com/MorpheusXAUT/vacs/commit/ad36dc55e2e42619eff9c0163e869f64910998bb))
* **vacs-client:** load ICE config after signaling connect ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))
* **vacs-webrtc:** use shared IceConfig types ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))

## [0.3.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-signaling-v0.2.1...vacs-signaling-v0.3.0) (2025-11-14)


### Features

* add protocol version to websocket login ([e778af9](https://github.com/MorpheusXAUT/vacs/commit/e778af94f3c50f807713a41b4c2553a79f82e8d6))
* implement automatic reconnecting in SignalingClient ([0e71616](https://github.com/MorpheusXAUT/vacs/commit/0e716163e766064e43069474f6024550d564c3b5)), closes [#3](https://github.com/MorpheusXAUT/vacs/issues/3)
* implement basic rate limiting ([e814366](https://github.com/MorpheusXAUT/vacs/commit/e814366e4aeb96b7ea7f825f661bc2b8d03e3c64))
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

## [0.2.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-signaling-v0.1.0...vacs-signaling-v0.2.0) (2025-11-09)


### Features

* implement basic rate limiting ([e814366](https://github.com/MorpheusXAUT/vacs/commit/e814366e4aeb96b7ea7f825f661bc2b8d03e3c64))
