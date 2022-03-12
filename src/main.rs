use std::env;
use std::thread;
use std::error::Error;
use std::fmt::{Display, Result as FmtResult};

use reqwest::{Result as ReqwestResult, Response as ReqwestResponce};
use tokio::sync::mpsc::{channel, Receiver, Sender};


#[derive(Debug)]
enum CurlErr {
    Get(String),
    Text(String),
}
impl Error for CurlErr {}
impl Display for CurlErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        match self {
            CurlErr::Get(msg) => write!(f, "wprobe get error: {}", msg),
            CurlErr::Text(msg) => write!(f, "wprobe text parse error: {}", msg),
        }
    }
}


#[tokio::main(worker_threads = 1)]
async fn main_tokio(mut rx: Receiver<String>, tx: Sender<Result<String, CurlErr>>) {
    while let Some(url) = rx.recv().await {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let result = make_request(url).await;
            if let Err(_) = tx_clone.send(result).await { return; }
        });
    }
}
async fn make_request(url: String) -> Result<String, CurlErr> {
    let body = reqwest::get(url)
    .await.map_err(|err| CurlErr::Get(err.to_string()))?
    .text()
    .await.map_err(|err| CurlErr::Text(err.to_string()))?;
    Ok(body)
}


fn main() {
    let mut arg_vec: Vec<String> = env::args().collect();

    let (tx_client, mut rx_main) = channel::<Result<String, CurlErr>>(4);
    let (tx_main, rx_client) = channel::<String>(4);
    
    thread::spawn(move || main_tokio(rx_client, tx_client));

    let url_vec = arg_vec.drain(1..);
    let mut url_count = url_vec.len();
    for url in url_vec {
        if let Err(err) = tx_main.blocking_send(url) {
            eprintln!("[FATAL]: {}", err);
            return;
        }
    }

    while url_count > 0 {
        if let Some(res) = rx_main.blocking_recv() {
            match res {
                Ok(text) => println!("[DONE]: {}", text),
                Err(err) => println!("[FAIL]: {}", err),
            }
        } else {
            return;
        }
        url_count -= 1;
    }

}
