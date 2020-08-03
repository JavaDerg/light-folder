FROM rust:1-buster
WORKDIR /usr/src/light-folder

RUN cargo init

COPY Cargo.toml .
COPY install_dependencies.sh .
RUN sudo ./install_dependencies.sh
RUN cargo build

COPY . .
RUN cargo install --path .

CMD ["light-folder"]
