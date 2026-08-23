use super::App;
use crate::game::Interaction;
use crate::network::client::send_interaction;

impl App {
    /// Fires `interaction` off the event loop when connection details are present, reporting
    /// whether it was sent. The response body carries no state; results arrive via the SSE
    /// `NetworkEvent` channel, so the send doesn't need to block a key handler.
    pub fn send_interaction_async(&self, interaction: Interaction) -> bool {
        let (Some(url), Some(client_id)) = (
            self.connection.server_url.clone(),
            self.connection.client_id.clone(),
        ) else {
            return false;
        };
        tokio::spawn(async move {
            if let Err(err) = send_interaction(&url, &client_id, &interaction).await {
                tracing::warn!(%err, "failed to send interaction");
            }
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_false_without_connection_details() {
        let app = App::new(false);
        assert!(!app.send_interaction_async(Interaction::Look));
    }
}
