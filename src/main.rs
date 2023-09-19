use std::io::Read;
use std::io::Result;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::windows::process;
use std::sync::Arc;
use std::time::Duration;
use flume::{bounded, Receiver, Sender};
use futures_lite::FutureExt;
use game_functions::{Block};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tungstenite::protocol::Role;
use tungstenite::{accept, Message, WebSocket};
use serde::Serialize;
use serde::Deserialize;

use crate::game_functions::Ball;
use crate::game_functions::Rect;
mod game_functions;

use uuid::Uuid;

const BLOCK_SIZE: u32 = 5;


#[derive(Debug, Serialize, Deserialize, Clone)]
struct Client {
    addr: SocketAddr,
    block: Block,
    sent: bool,
    id: String
}

#[derive(Debug, Serialize, Deserialize)]
struct Game {
    score: (u32,u32),
    tick: u64,
    clients: Vec<Client>,
    is_started: bool,
    ball: Block,
    headers_sent: bool
}

const PORT: u16 = 9001;

async fn update_game(game: Arc<Mutex<Game>>){
    let lock = game.lock().await;
    
}

async fn handle_writing(
    mut write: WebSocket<PanicOnRead<TcpStream>>,
    addr: SocketAddr,
    rx: Receiver<Message>,
    game: Arc<Mutex<Game>>,
) {
    println!("Starting sender for: {:?}", addr);
    let mut count = 0;
    let game = Arc::clone(&game);
    let mut headers_sent = false;
    let msg = Message::Text(addr.to_string());
    write.send(msg).unwrap();
    loop {
        // Check if there are at least 2 clients to start the game
        {
            let mut lock: tokio::sync::MutexGuard<'_, Game> = game.lock().await;
           
            if lock.clients.len() < 2 {
                if lock.is_started {
                    println!("Other client disconnected! Game is exiting.");
                    std::process::exit(0);
                } else {
                    lock.is_started = false;
                    println!("Game needs at least 2 clients to start playing");
                    // let msg = Message::Text("Game waiting for other client".to_string());
                    // if let Err(e) = write.write_message(msg) {
                    //     println!("Error sending message to client {}: {:?}", addr, e);
                    //     break;
                    // }
                    tokio::time::sleep(Duration::from_micros(15625)).await;
                    continue;
                }
            }
            lock.is_started = true;
            
           
        }
        
        {
            let mut lock = game.lock().await;
            if !lock.headers_sent {
                let msg = Message::Text(addr.to_string());
                write.send(msg).unwrap();
                lock.headers_sent = true;
            }
        }
        
        //update game mechanincs
        {
            let mut lock = game.lock().await;
            let mut clients = lock.clients.clone();
            let mut score = lock.score.clone();

            for client in &mut clients{
                client.block.update_position();
                let client_block = client.block.clone();   
                lock.ball.handle_wall();
                lock.ball.react_object(&client_block);
            }
            lock.ball.handle_score(&mut score);
            lock.ball.update_position();
        }
       
        
        
        

        // Construct the JSON payload
        let json = {
            let lock = game.lock().await;
            serde_json::to_string(&*lock).unwrap()
        };
        let msg = Message::Text(json);

        // Send the message to all clients
        if let Err(e) = write.write_message(msg) {
            println!("Error sending message to client {}: {:?}", addr, e);
            break;
        }

        // Mark the current client as sent
        {
            let mut lock = game.lock().await;
            for client in &mut lock.clients {
                if client.addr == addr {
                    client.sent = true;
                    break;
                }
            }
        }

        count += 1;
        tokio::time::sleep(Duration::from_micros(15625)).await;

        // Check if all clients have sent their data and update the tick
        {
            let mut lock = game.lock().await;
            let mut sent_clients_count = 0;
            for client in &lock.clients {
                if client.sent {
                    sent_clients_count += 1;
                }
            }
            if sent_clients_count == lock.clients.len() {
                lock.tick += 1;
                // Reset the 'sent' flag for all clients
                for client in &mut lock.clients {
                    client.sent = false;
                }
            }
        }
    }

    println!("Stopping sender for: {:?}", addr);
}
async fn handle_client(
    read: WebSocket<PanicOnWrite<TcpStream>>,
    write: WebSocket<PanicOnRead<TcpStream>>,
    game: Arc<Mutex<Game>>,
    addr: SocketAddr,
) {
    println!("a client connected, addr: {}", addr);

    // Kanallar oluştur
    let (tx,  rx) = bounded(100); // 100 kapasiteli kanal

    // Mesajları gönderen ve alıcı iş parçacığı

    let reader = tokio::spawn(
        handle_read(addr, tx, read, game.clone())
    );
    let sender = tokio::spawn(handle_writing(write, addr, rx, game.clone()));
    
    //println!("{:?}", reader.race(sender).await);
}

async fn handle_read(addr: SocketAddr, tx: Sender<Message>, mut read: WebSocket<PanicOnWrite<TcpStream>>, mut game: Arc<Mutex<Game>>) {
    loop {
        let msg = match read.read_message() {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error reading message from client: {:?}", e);
                break;
            }
        };
        
        // We do not want to send back ping/pong messages.
        if msg.is_text() {
            let mut lock = game.lock().await;
    
            for mut client in lock.clients.iter_mut(){
                if (client.addr == addr) {

                    let json_value: Value = match serde_json::from_str(msg.to_string().as_str()) {
                        Ok(value) => value,
                        Err(e) => {
                            let err_message = format!("Error parsing JSON: {:?}", e);
                            println!("{}", err_message);
                            let msg = Message::Text(err_message);
                            tx.send(msg).unwrap();
                            continue;
                        }
                    };
            
                    // Serde ile deserialize ederek veriyi Block türüne dönüştür
                    let block: Block = match serde_json::from_value(json_value) {
                        Ok(block) => block,
                        Err(e) => {
                            let err_message = format!("Error deserializing to Block: {:?}", e);
                            println!("{}", err_message);
                            let msg = Message::Text(err_message);
                            tx.send(msg).unwrap();
                            continue;
                        }
                    };

                    client.block = block;
                    println!("changed : {:?}", client );
                }
            }
        } else if msg.is_close() {
            let mut game = game.lock().await;
            if let Some(pos) = game.clients.iter().position(|c| c.addr == addr) {
                game.clients.remove(pos);
                println!("Client disconnected: {:?}", addr);
            }
            break;
        }
    }
}

pub struct PanicOnWrite<S>(pub S);
impl<S: Read> Read for PanicOnWrite<S> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }
}
impl<S: Write> Write for PanicOnWrite<S> {
    fn write(&mut self, _buf: &[u8]) -> Result<usize> {
        panic!("called write")
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct PanicOnRead<S>(pub S);
impl<S> Read for PanicOnRead<S> {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        panic!("called read")
    }
}
impl<S: Write> Write for PanicOnRead<S> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.0.flush()
    }
}

/// A WebSocket echo server
#[tokio::main(flavor = "multi_thread",worker_threads =10)]
async fn main() {
    let server = TcpListener::bind(format!("0.0.0.0:{}", PORT)).unwrap();

    //declare ball
    let ball_rect = Rect::new(40, 40, BLOCK_SIZE*10, BLOCK_SIZE*10);
    let ball = Block::new(ball_rect, BLOCK_SIZE as i32 *5, BLOCK_SIZE as i32 *1);

    //declare game 
    let game: Arc<Mutex<Game>> = Arc::new(Mutex::new(Game {
        tick: 0,
        clients: vec![],
        is_started: false,
        ball: ball,
        score: (0,0),
        headers_sent: false
    }));

    println!("binded to port: {}", PORT);

    for stream in server.incoming() {
        let addr = stream.as_ref().unwrap().peer_addr().unwrap();
        let rect = game_functions::Rect::new(750, 750, BLOCK_SIZE*24, BLOCK_SIZE*3);
        let block = Block::new(rect, 50, 50);

        let id = Uuid::new_v4().to_string();
        let client_id = id.clone();

        let client = Client {
            addr,
            block: block,
            sent: false,
            id: id
        };

        {
            // Lock the game for writing and update the clients vector
            let mut game = game.lock().await;
            game.clients.push(client);
        }

        let stream = stream.unwrap();
        let write = PanicOnRead(stream.try_clone().unwrap());
        let read = PanicOnWrite(stream.try_clone().unwrap());
        // Run the handshake
        accept(stream).unwrap();
        
        let read_ws = WebSocket::from_raw_socket(read, Role::Server, None);
        let mut write_ws = WebSocket::from_raw_socket(write, Role::Server, None);
        let game_clone = game.clone();

        tokio::spawn(
            handle_client(read_ws, write_ws, game_clone, addr)
        );
    }

}