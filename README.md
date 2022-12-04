# amykia

Amykia is a multi-function personal web service and user interface for creating and accessing content. 


## Usage

Example commands for how to build and run the Docker container for this project:
```sh
docker build -t amykia:latest . && docker image prune -f
# Replace `/home/bob/stuff/things` with a directory of your choosing.
docker run -d -p 5000:5000 --name amk -v /home/bob/stuff/things:/public amykia:latest
```

To turn it off and remove the container:
```sh
docker stop amk
docker rm amk
```

The image is planned to be listed on the GitHub Container Registry.
