use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

fn main() {
    // 在本地主机的 7878 端口监听传入的 TCP 连接
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    // 等待连接，并在当前线程逐个处理
    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    // 单次最多读取 1024 字节；TCP 可能只返回请求的一部分
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).unwrap();
    if n == 0 {
        return;
    }

    let get = b"GET / HTTP/1.1\r\n";

    // 根据请求中的数据，返回问候页面或 404 页面
    let (status_line, filename) = if buffer[..n].starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
    };
    let contents = fs::read_to_string(filename).unwrap();

    // 完整写入响应字节；flush 不保证对端已经收到或处理响应
    let response = format!("{status_line}{contents}");
    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
