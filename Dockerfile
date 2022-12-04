FROM rust:latest AS build

# Make a new Cargo project inside the image and copy in the Cargo files.
RUN cargo new amykia
WORKDIR /amykia
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# Build (and cache) just the dependencies.
RUN cargo build --release
RUN rm src/*.rs

# Copy the source code files into the image.
COPY ./src ./src

# Install the binary on the system path.
RUN cargo install --path .

# Use a simple, Rust-less image to run just the Linux binary itself.
FROM debian:buster-slim
COPY --from=build /amykia/target/release/amykia .
VOLUME [ "/public" ]
EXPOSE 5000
CMD ["./amykia", "0.0.0.0:5000", "8"]