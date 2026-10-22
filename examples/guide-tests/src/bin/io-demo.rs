use async_guide_tests::io::{read_frame, write_frame};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (mut writer, mut reader) = tokio::io::duplex(8);
    let ((), message) = tokio::try_join!(
        async {
            write_frame(&mut writer, b"hello async IO").await?;
            tokio::io::AsyncWriteExt::shutdown(&mut writer).await
        },
        read_frame(&mut reader)
    )?;
    println!("{}", String::from_utf8_lossy(&message));
    Ok(())
}
