use crate::error::{SignalingError, SignalingRuntimeError};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tracing::instrument;
use vacs_protocol::ws::SignalingMessage;

/// Represents a waiting request for a message that matches a predicate.
struct MatcherEntry {
    predicate: Box<dyn Fn(&SignalingMessage) -> bool + Send + Sync + 'static>,
    responder: oneshot::Sender<SignalingMessage>,
}

/// ResponseMatcher holds a queue of waiters that want to match an incoming message.
#[derive(Clone, Default)]
pub struct ResponseMatcher {
    /// Queue of matcher entries waiting for a specific message pattern.
    /// Note: Each matcher is served only once per message fan-out.
    inner: Arc<Mutex<VecDeque<MatcherEntry>>>,
}

impl ResponseMatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Waits for an incoming message to match the given predicate with a timeout.
    /// Entries are evaluated in order of appearance and removed from the internal queue in case of a match.
    /// Only the first successful matcher will receive the message.
    ///
    /// # Returns
    ///
    /// - `Ok(Message)` if a matching message was received within the timeout.
    /// - `Err(SignalingError:Timeout)` if the timeout was reached before a matching message was received.
    /// - `Err(SignalingError:Disconnected)` if the Matcher was closed unexpectedly.
    #[instrument(level = "debug", skip(self, predicate), err)]
    pub async fn wait_for_with_timeout<F>(
        &self,
        predicate: F,
        timeout: Duration,
    ) -> Result<SignalingMessage, SignalingError>
    where
        F: Fn(&SignalingMessage) -> bool + Send + Sync + 'static,
    {
        let (tx, rx) = oneshot::channel();

        let entry = MatcherEntry {
            predicate: Box::new(predicate),
            responder: tx,
        };

        self.inner.lock().await.push_back(entry);

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(_)) => Err(SignalingError::Runtime(SignalingRuntimeError::Disconnected)),
            Err(_) => Err(SignalingError::Timeout("Matcher timed out".to_string())),
        }
    }

    /// Waits for an incoming message to match the given predicate until one has been received.
    /// Entries are evaluated in order of appearance and removed from the internal queue in case of a match.
    /// Only the first successful matcher will receive the message.
    ///
    /// This internally uses [`wait_for_with_timeout`] and waits for [`std::time::Duration::MAX`],
    /// which results in approximately 584,942,417,355 years of wait time - probably enough for our use cases.
    ///
    /// # Returns
    ///
    /// - `Ok(Message)` if a matching message was received within the timeout.
    /// - `Err(SignalingError:Timeout)` if the timeout was reached before a matching message was received.
    ///     Should you ever make it here, please open an issue with the next generation of project maintainers.
    /// - `Err(SignalingError:Disconnected)` if the Matcher was closed unexpectedly.
    #[instrument(level = "debug", skip(self, predicate), err)]
    pub async fn wait_for<F>(&self, predicate: F) -> Result<SignalingMessage, SignalingError>
    where
        F: Fn(&SignalingMessage) -> bool + Send + Sync + 'static,
    {
        self.wait_for_with_timeout(predicate, Duration::MAX).await
    }

    /// Called by the receiving task to check if a message completes any match. If so, the message is
    /// forwarded to the matcher awaiting it and not processed any further by [`try_match`].
    #[instrument(level = "debug", skip(self, msg))]
    pub fn try_match(&self, msg: &SignalingMessage) {
        let mut inner = self.inner.try_lock();
        if let Ok(ref mut queue) = inner
            && let Some(pos) = queue.iter().position(|entry| (entry.predicate)(msg))
            && let Some(entry) = queue.remove(pos)
        {
            let _ = entry.responder.send(msg.clone());
        }
    }

    /// Clears all currently stored matchers.
    /// This should be called when the transport is disconnected/reset to a clean state to avoid
    /// an inconsistent consumer state.
    pub async fn clear(&self) {
        self.inner.lock().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_matches;
    use test_log::test;
    use vacs_protocol::ws::ClientInfo;

    #[test(tokio::test)]
    async fn wait_for() {
        let matcher = ResponseMatcher::new();

        let matcher_clone = matcher.clone();
        let handle = tokio::spawn(async move {
            matcher_clone
                .wait_for(|msg| matches!(msg, SignalingMessage::Logout))
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        matcher.try_match(&SignalingMessage::Logout);

        let result = handle.await.unwrap();
        assert_matches!(result, Ok(SignalingMessage::Logout));
    }

    #[test(tokio::test)]
    async fn wait_for_content() {
        let matcher = ResponseMatcher::new();
        let msg = SignalingMessage::ClientList {
            clients: vec![ClientInfo {
                id: "client1".to_string(),
                display_name: "Client 1".to_string(),
                frequency: "100.000".to_string(),
            }],
        };

        let matcher_clone = matcher.clone();
        let handle = tokio::spawn(async move {
            matcher_clone
                .wait_for(|msg| matches!(msg, SignalingMessage::ClientList { .. }))
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        matcher.try_match(&msg);

        let result = handle.await.unwrap();
        assert_matches!(result, Ok(SignalingMessage::ClientList { clients }) if clients.len() == 1);
    }

    #[test(tokio::test)]
    async fn wait_for_matching_peer_id() {
        let matcher = ResponseMatcher::new();
        let messages = vec![
            SignalingMessage::CallAnswer {
                peer_id: "client1".to_string(),
                sdp: "sdp1".to_string(),
            },
            SignalingMessage::CallAnswer {
                peer_id: "client2".to_string(),
                sdp: "sdp2".to_string(),
            },
            SignalingMessage::CallAnswer {
                peer_id: "client3".to_string(),
                sdp: "sdp3".to_string(),
            },
        ];

        let matcher_clone = matcher.clone();
        let handle = tokio::spawn(async move {
            matcher_clone
                .wait_for(|msg| matches!(msg, SignalingMessage::CallAnswer { peer_id, .. } if peer_id == "client2"))
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        for msg in messages {
            matcher.try_match(&msg);
        }

        let result = handle.await.unwrap();
        assert_matches!(result, Ok(SignalingMessage::CallAnswer { peer_id, sdp }) if peer_id == "client2" && sdp == "sdp2");
    }

    #[test(tokio::test)]
    async fn wait_for_with_timeout() {
        let matcher = ResponseMatcher::new();
        let msg = SignalingMessage::Logout;

        let matcher_clone = matcher.clone();
        let handle = tokio::spawn(async move {
            matcher_clone
                .wait_for_with_timeout(
                    |msg| matches!(msg, SignalingMessage::Logout),
                    Duration::from_millis(100),
                )
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        matcher.try_match(&msg);

        let result = handle.await.unwrap();
        assert_matches!(result, Ok(SignalingMessage::Logout));
    }

    #[test(tokio::test)]
    async fn wait_for_with_timeout_expires() {
        let matcher = ResponseMatcher::new();

        let result = matcher
            .wait_for_with_timeout(
                |msg| matches!(msg, SignalingMessage::Logout),
                Duration::from_millis(1),
            )
            .await;
        assert_matches!(result, Err(SignalingError::Timeout(_)));
    }

    #[test(tokio::test)]
    async fn try_match_matches_only_once() {
        let matcher = ResponseMatcher::new();

        let m1 = matcher.clone();
        let m2 = matcher.clone();

        let h1 = tokio::spawn(async move {
            m1.wait_for_with_timeout(
                |m| matches!(m, SignalingMessage::Logout),
                Duration::from_millis(20),
            )
            .await
        });
        let h2 = tokio::spawn(async move {
            m2.wait_for_with_timeout(
                |m| matches!(m, SignalingMessage::Logout),
                Duration::from_millis(20),
            )
            .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        matcher.try_match(&SignalingMessage::Logout);

        let r1 = h1.await.unwrap();
        let r2 = h2.await.unwrap();

        // One should succeed, the other one should time out
        assert!(
            matches!(r1, Ok(SignalingMessage::Logout)) ^ matches!(r2, Ok(SignalingMessage::Logout))
        );
    }

    #[test(tokio::test)]
    async fn try_match_with_overlapping_predicates() {
        let matcher = ResponseMatcher::new();

        let m1 = matcher.clone();
        let m2 = matcher.clone();

        let h1 = tokio::spawn(async move {
            m1.wait_for_with_timeout(
                |m| {
                    matches!(
                        m,
                        SignalingMessage::Logout | SignalingMessage::PeerNotFound { .. }
                    )
                },
                Duration::from_millis(20),
            )
            .await
        });
        let h2 = tokio::spawn(async move {
            m2.wait_for_with_timeout(
                |m| matches!(m, SignalingMessage::Logout),
                Duration::from_millis(20),
            )
            .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        matcher.try_match(&SignalingMessage::Logout);

        let r1 = h1.await.unwrap();
        let r2 = h2.await.unwrap();

        let matches = [r1, r2]
            .iter()
            .filter(|r| matches!(r, Ok(SignalingMessage::Logout)))
            .count();
        assert_eq!(matches, 1);
    }

    #[test(tokio::test)]
    async fn try_match_concurrent_waiters() {
        let matcher = ResponseMatcher::new();

        let barrier = Arc::new(tokio::sync::Barrier::new(11));
        let mut handles = vec![];

        for _ in 0..10 {
            let matcher_clone = matcher.clone();
            let barrier_clone = barrier.clone();
            handles.push(tokio::spawn(async move {
                barrier_clone.wait().await;
                matcher_clone
                    .wait_for_with_timeout(
                        |m| matches!(m, SignalingMessage::Logout),
                        Duration::from_millis(20),
                    )
                    .await
            }));
        }

        barrier.wait().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        matcher.try_match(&SignalingMessage::Logout);

        let mut matches = 0;
        for h in handles {
            if matches!(h.await.unwrap(), Ok(SignalingMessage::Logout)) {
                matches += 1;
            }
        }
        assert_eq!(matches, 1);
    }

    #[test(tokio::test)]
    async fn try_match_burst() {
        let matcher = ResponseMatcher::new();

        let h1 = matcher.clone();
        let h2 = matcher.clone();

        let w1 = tokio::spawn(async move {
            h1.wait_for(|msg| matches!(msg, SignalingMessage::CallAnswer { .. }))
                .await
        });

        let w2 = tokio::spawn(async move {
            h2.wait_for(|msg| matches!(msg, SignalingMessage::ClientList { .. }))
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        for _ in 0..10 {
            matcher.try_match(&SignalingMessage::Logout);
        }

        matcher.try_match(&SignalingMessage::ClientList {
            clients: vec![ClientInfo {
                id: "client1".into(),
                display_name: "Client 1".into(),
                frequency: "100.000".into(),
            }],
        });
        matcher.try_match(&SignalingMessage::CallAnswer {
            peer_id: "client2".to_string(),
            sdp: "sdp2".into(),
        });

        let r1 = w1.await.unwrap();
        let r2 = w2.await.unwrap();

        assert_matches!(r1, Ok(SignalingMessage::CallAnswer { .. }));
        assert_matches!(r2, Ok(SignalingMessage::ClientList { .. }));
    }

    #[test(tokio::test)]
    async fn try_match_without_matchers() {
        let matcher = ResponseMatcher::new();
        matcher.try_match(&SignalingMessage::Logout);
    }
}
