# Changelog

## [1.1.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-client-v1.0.0...vacs-client-v1.1.0) (2025-11-30)


### Features

* provide TURN servers for traversing restrictive networks ([#248](https://github.com/MorpheusXAUT/vacs/issues/248)) ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))
* **vacs-client:** add profile select to mission page ([ad36dc5](https://github.com/MorpheusXAUT/vacs/commit/ad36dc55e2e42619eff9c0163e869f64910998bb))
* **vacs-client:** add station filter and aliasing ([#233](https://github.com/MorpheusXAUT/vacs/issues/233)) ([ad36dc5](https://github.com/MorpheusXAUT/vacs/commit/ad36dc55e2e42619eff9c0163e869f64910998bb))
* **vacs-client:** implement dial pad functionality ([#231](https://github.com/MorpheusXAUT/vacs/issues/231)) ([3e6b03d](https://github.com/MorpheusXAUT/vacs/commit/3e6b03d573ce8e2fb1816177da5ca750cc3a8fe1))
* **vacs-client:** Implement Fullscreen functionality ([#223](https://github.com/MorpheusXAUT/vacs/issues/223)) ([288965e](https://github.com/MorpheusXAUT/vacs/commit/288965e95c683b46d4b9d15aeb74d8207416561f))
* **vacs-client:** load ICE config after signaling connect ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))
* **vacs-server:** implement GitHub release catalog ([#258](https://github.com/MorpheusXAUT/vacs/issues/258)) ([6dac184](https://github.com/MorpheusXAUT/vacs/commit/6dac18498899760e654fe7485bce4944a8a723ac))
* **vacs-webrtc:** use shared IceConfig types ([e4b8b91](https://github.com/MorpheusXAUT/vacs/commit/e4b8b91320fd6d072ef4ba1c98de56ad14c8dcfe))


### Bug Fixes

* **vacs-client:** remove spammy updater progress log ([6dac184](https://github.com/MorpheusXAUT/vacs/commit/6dac18498899760e654fe7485bce4944a8a723ac))

## [1.0.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-client-v0.4.0...vacs-client-v1.0.0) (2025-11-14)


### Bug Fixes

* **vacs-client:** fix DA key overflow and sorting ([#204](https://github.com/MorpheusXAUT/vacs/issues/204)) ([c1b2da5](https://github.com/MorpheusXAUT/vacs/commit/c1b2da5e39126b033fa24251eb725001c244080a))

## [0.4.0](https://github.com/MorpheusXAUT/vacs/compare/vacs-client-v0.3.0...vacs-client-v0.4.0) (2025-11-12)


### Features

* implement basic rate limiting ([e814366](https://github.com/MorpheusXAUT/vacs/commit/e814366e4aeb96b7ea7f825f661bc2b8d03e3c64))
* **vacs-client:** add auto-hangup for unanswered calls ([4f32f22](https://github.com/MorpheusXAUT/vacs/commit/4f32f22877371eaa10045f94d664aa1a81afcee3))
* **vacs-client:** add keybind requirements to macos app info ([32a5508](https://github.com/MorpheusXAUT/vacs/commit/32a55083594c192ced098aef8c5d8a3496686e11))
* **vacs-client:** add macos keybinds emitter runtime ([7ed239f](https://github.com/MorpheusXAUT/vacs/commit/7ed239f2d4f94265e7a590c1f2923ca939646ebb))
* **vacs-client:** add macos keybinds listener runtime ([1be1cdf](https://github.com/MorpheusXAUT/vacs/commit/1be1cdf3b257086c03c621c5109718eae1c5397a))
* **vacs-client:** customize nsis installer ([abf4bb0](https://github.com/MorpheusXAUT/vacs/commit/abf4bb04ca16c75128514a2750595c5498689f99))
* **vacs-client:** increase default auto hangup timeout to 60s ([e03fa84](https://github.com/MorpheusXAUT/vacs/commit/e03fa848600756f1809872491d06101b0e3d6bd6))
* **vacs-client:** prevent default browser shortcuts ([24ac82f](https://github.com/MorpheusXAUT/vacs/commit/24ac82fc2e59fb7670c610c1c1a5e8e374057629))


### Bug Fixes

* **vacs-client:** add microphone access request for macos ([7a88e9b](https://github.com/MorpheusXAUT/vacs/commit/7a88e9b092861f71285041a10cc528a49967eadb))
* **vacs-client:** fix app icon size for macos ([cb9aa81](https://github.com/MorpheusXAUT/vacs/commit/cb9aa81baeca819eb07e2bb7a53039907b0fdc60))
* **vacs-client:** fix call queue and DA key labels ([22f350b](https://github.com/MorpheusXAUT/vacs/commit/22f350b120e591ea7e6a5e08f562b989e69feee3))
* **vacs-client:** fix deep link handling for macos ([6a2fc95](https://github.com/MorpheusXAUT/vacs/commit/6a2fc95a96cbe2844d7fb031f5ba824162c47ad1))
* **vacs-client:** fix default window size for macos ([97de5dd](https://github.com/MorpheusXAUT/vacs/commit/97de5dd4444b5f468b3b4508b82cc4b4d53c11d6))
* **vacs-client:** fix font synthesis for macos ([46c09d8](https://github.com/MorpheusXAUT/vacs/commit/46c09d85e6b6f375c6785270ae89e4c2cfa54a72))
* **vacs-client:** fix login page loading state ([75b812f](https://github.com/MorpheusXAUT/vacs/commit/75b812fd58a0c4a3cc653231c18fe271aff920a4))
* **vacs-client:** fix login page loading state ([4813ebd](https://github.com/MorpheusXAUT/vacs/commit/4813ebd0d1feaaa66e743fcc80989f168a49e811))
* **vacs-client:** fix macos select height ([02b3576](https://github.com/MorpheusXAUT/vacs/commit/02b35767ae07ac914c6e764ac9cc1feaa6376c74))
* **vacs-client:** fix remove peer behaviour in frontend state ([c37d3b9](https://github.com/MorpheusXAUT/vacs/commit/c37d3b99fc4ba1a615a019dc78ddbd59d12e734f))
* **vacs-client:** fix unavailable keybinds settings ui ([6e692ae](https://github.com/MorpheusXAUT/vacs/commit/6e692ae061bbbc185dfedcc2eece28cd65339ee6))
