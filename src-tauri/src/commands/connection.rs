use serde::Serialize;
use tauri::{AppHandle, State};

use crate::connection::remote::probe_health;
use crate::connection::transport::http::HttpTransport;
use crate::connection::types::{CommandError, ConnectionConfig, Emitter};
use crate::connection::ConnectionManager;

fn conn_err(msg: String) -> CommandError {
	CommandError {
		code: 503,
		message: msg,
	}
}

async fn emit_connection_changed(app: &AppHandle, state: &ConnectionManager) {
	let (connections, active_id) = state.get_connections(app).await;
	app.emit_connection_changed(serde_json::json!({
		"connections": connections,
		"active_id": active_id,
	}));
}

#[derive(Serialize)]
pub struct ConnectionsState {
	pub connections: Vec<ConnectionConfig>,
	pub active_id: String,
}

#[tauri::command]
pub async fn get_connections(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
) -> Result<ConnectionsState, CommandError> {
	let (connections, active_id) = state.get_connections(&app).await;
	Ok(ConnectionsState {
		connections,
		active_id,
	})
}

/// Stage one of adding a remote: is anything answering at this URL at all,
/// before the user is asked for a pairing code that only the right daemon can
/// even accept. Tokenless and stateless -- `/v0/health` needs no auth, and
/// this candidate is not yet, and may never become, a saved connection.
#[tauri::command]
pub async fn check_remote_health(url: String) -> Result<(), CommandError> {
	let transport = HttpTransport::new(url, None);
	probe_health(&transport).await.map_err(conn_err)
}

#[tauri::command]
pub async fn add_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	name: String,
	url: String,
	code: String,
) -> Result<ConnectionConfig, CommandError> {
	let config = state
		.add_connection(&app, name, url, code)
		.await
		.map_err(conn_err)?;
	emit_connection_changed(&app, &state).await;
	Ok(config)
}

#[tauri::command]
pub async fn remove_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
) -> Result<(), CommandError> {
	state.remove_connection(&app, &id).await.map_err(conn_err)?;
	emit_connection_changed(&app, &state).await;
	Ok(())
}

#[tauri::command]
pub async fn switch_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
) -> Result<(), CommandError> {
	state.switch_connection(&app, &id).await.map_err(conn_err)?;
	Ok(())
}

#[tauri::command]
pub async fn rename_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
	name: String,
) -> Result<(), CommandError> {
	state.rename_connection(&app, &id, name)
		.await
		.map_err(conn_err)?;
	emit_connection_changed(&app, &state).await;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::transport::testing::drain_request;
	use tokio::io::AsyncWriteExt;
	use tokio::net::TcpListener;

	#[tokio::test]
	async fn check_remote_health_succeeds_against_a_reachable_peer() {
		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		tokio::spawn(async move {
			if let Ok((mut sock, _)) = listener.accept().await {
				drain_request(&mut sock).await;
				let _ = sock
					.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
					.await;
			}
		});

		check_remote_health(format!("tcp://{addr}"))
			.await
			.expect("a reachable peer must pass the health check");
	}

	#[tokio::test]
	async fn check_remote_health_names_the_status_on_a_bad_peer() {
		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		tokio::spawn(async move {
			if let Ok((mut sock, _)) = listener.accept().await {
				drain_request(&mut sock).await;
				let _ = sock
					.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
					.await;
			}
		});

		let err = check_remote_health(format!("tcp://{addr}"))
			.await
			.expect_err("a 404 must not pass the health check");
		assert!(err.message.contains("404"), "got {err:?}");
	}

	#[tokio::test]
	async fn check_remote_health_fails_when_nothing_is_listening() {
		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		drop(listener);

		let err = check_remote_health(format!("tcp://{addr}"))
			.await
			.expect_err("an unreachable host must not pass the health check");
		assert!(!err.message.is_empty());
	}
}
