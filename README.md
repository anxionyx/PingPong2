# PingPong
## My Second Ping Pong game made with rust to relax in terminal
> [!NOTE]
> Branched from PingPong from the same maintainer
### Installation
#### Method 1 : Manual Build
Run the following commands
```
git clone https://github.com/anxionyx/PingPong.git 
cd PingPong
cargo build --release
./target/release/PingPong
```
(For Linux,Optional)
```
cp ./target/release/PingPong /usr/bin/pingpong
pingpong
```
#### Method 2
Go to the release page and download the executable  
(New Releases will be auto generated for each commit)
#### Method 3 (For Termux)
```
wget -O $PREFIX/bin/pp  https://github.com/anxionyx/PingPong/releases/download/v0.1.9/PingPong-termux && chmod +x $PREFIX/bin/pp
```
> [!WARNING]
> This uses the leagacy version from the original PingPong game. It is very old but playable
> [!TIP]
> For the bleeding edge game experience , do "pkg i rust" and compile using Method 1
