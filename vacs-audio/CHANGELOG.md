# Changelog

## 0.1.0 (2025-10-30)


### Features

* add transmit mode to capture stream ([d2a8715](https://github.com/MorpheusXAUT/vacs/commit/d2a8715bd15efcbe912d0e292120db0844de36aa))
* **vacs-audio:** add amp to audio output and input ([f2a07e4](https://github.com/MorpheusXAUT/vacs/commit/f2a07e48a71b6f997b39b555248dfa8ba3720c62))
* **vacs-audio:** add discrete types for audio start and runtime errors ([64649e9](https://github.com/MorpheusXAUT/vacs/commit/64649e9235738d4f731b361d1cb6c77613692807))
* **vacs-audio:** add functionality for retrieving audio hosts ([584bf02](https://github.com/MorpheusXAUT/vacs/commit/584bf0279fa3ec8d14a1ae3b4365cbfeda0e5460))
* **vacs-audio:** add input level meter for displaying volume feedback ([8bcce1e](https://github.com/MorpheusXAUT/vacs/commit/8bcce1e3475c29ec172722f57803e3bf76747a51))
* **vacs-audio:** implement AudioDevice ([654699a](https://github.com/MorpheusXAUT/vacs/commit/654699a87e2d7ed7276d0c6922c38f316ce81963))
* **vacs-audio:** implement AudioInput ([b29b839](https://github.com/MorpheusXAUT/vacs/commit/b29b839d7847fc0fa88482a0b94653190df5dfc6))
* **vacs-audio:** implement DeviceSelector with improved device support ([5d3999a](https://github.com/MorpheusXAUT/vacs/commit/5d3999ae6ab833cfb52d82bb914632feb686ade9))
* **vacs-audio:** implement input CaptureStream ([4ca5fa0](https://github.com/MorpheusXAUT/vacs/commit/4ca5fa08644ade70c65c32c97784ed6cf1e3a8cc))
* **vacs-audio:** implement input level meter for capture stream ([c7b4279](https://github.com/MorpheusXAUT/vacs/commit/c7b427993286ee23da58b46f273b95909aa973bc))
* **vacs-audio:** implement mic processor for audio capture improvement ([3c42ead](https://github.com/MorpheusXAUT/vacs/commit/3c42ead83536dfbdce4a2b791d5061ded8b29272))
* **vacs-audio:** implement mixer for audio output ([4ef1083](https://github.com/MorpheusXAUT/vacs/commit/4ef108345a67eaeac486211e482967c733bab5b2))
* **vacs-audio:** implement Opus source for decoding audio received from webrtc ([7d2bd72](https://github.com/MorpheusXAUT/vacs/commit/7d2bd7239e380a431db6a98186e119f9e8a3d3e2))
* **vacs-audio:** implement output PlaybackStream ([807a7b8](https://github.com/MorpheusXAUT/vacs/commit/807a7b897c40f88815868a7b70cdce6195cb218a))
* **vacs-audio:** implement sample format conversion to/from f32 ([8745b88](https://github.com/MorpheusXAUT/vacs/commit/8745b885cb85aa680945063a3cf38c16629dd44a))
* **vacs-audio:** implement waveform audio source ([1cd72a1](https://github.com/MorpheusXAUT/vacs/commit/1cd72a1c03a37ac714f1600ad378bbfb22c7e81b))
* **vacs-audio:** pick best fallback audio device instead of first ([fcfc253](https://github.com/MorpheusXAUT/vacs/commit/fcfc2530543962f898513e8fb53dbb941724fc30))
* **vacs-audio:** separate input level meter into separate audio input ([0a4974f](https://github.com/MorpheusXAUT/vacs/commit/0a4974f96f79456bf89d8e3ae30a237e0dc27ece))
* **vacs-client:** list all available audio devices and supported configs on startup ([1d4bb85](https://github.com/MorpheusXAUT/vacs/commit/1d4bb85b138500bbb12f14150fd500cb664e5696))
* **vacs-client:** start implementing keybind selection ([175da47](https://github.com/MorpheusXAUT/vacs/commit/175da478391f5842ec38a371e9bf5ddae574c550))
* **vacs-client:** wip: wip: add input level meter to settings page ([9e0b15b](https://github.com/MorpheusXAUT/vacs/commit/9e0b15b2b4a860b57bdb6236c5bdb31eb292236a))
* wip: add audio device list functionality to client ([58ec721](https://github.com/MorpheusXAUT/vacs/commit/58ec7214b36ccb1c8207f31760373e4004ba58c5))


### Bug Fixes

* handle capture and playback stream errors during runtime ([f4beb66](https://github.com/MorpheusXAUT/vacs/commit/f4beb66bac002f735fed2e7c6d97bc96dd57f06a))
* **vacs-audio:** fix incorrect is_fallback for default devices ([53671c0](https://github.com/MorpheusXAUT/vacs/commit/53671c09a1cc3efb184bfe4b21c5055cadebd657))
* **vacs-audio:** fix opus decode buffer overflow logging ([9ff4978](https://github.com/MorpheusXAUT/vacs/commit/9ff497861ab6ff70f23abbeef2453bc7514d3745))
* **vacs-client:** switch output device on setting audio host ([aa31888](https://github.com/MorpheusXAUT/vacs/commit/aa3188887ddc07b8adfa4e2e2f0f255433cdae8f))
