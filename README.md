# Rust Daemon project
This project creates a daemon which reads in a config file config.json and automatically starts all the apps which have auto_start set as True in the config file as their own processess and then monitor them by holding onto their PID. 

The daemon PID is written to:
/tmp/daemon.pid

The daemon can be killed with the following command. 
```
kill -9 `cat /tmp/daemon.pid`
```
With out the daemon running the apps will eventually exit.


Each application should have the following config values:

     - name: The app name. Example: app1

     - autostart - Start automatically when the Daemon starts. Example: true

     - user - Run as this user. Example: root

     - working_dir - The working dir. Example: /home/ubuntu/app

     - command - The path to the executable. Example: /home/ubuntu/app/executable

     - stdout_logfile - The path to the stdout logfile. Example: /var/log/app

     - sdterr_logfile - The path to the stderr logfile. Example: /var/log/app.err.log

     - env - A list of env params to run with the file. Example: VAR1=2,VAR2=3,VAR3="smth"

After the config file is processed the deamon will start simultaneously all of the applications with autostart=true in the config file.
If an application is killed or exists the Daemon must auto start it again if autostart=true.

The daemon also supports a way for the CLI CTL app described bellow to connect and fetch stats or execute operations.
It does this by utilizing sockets for Inter Process Communication. The daemon opens a listening socket on localhost 
waiting to service CLI requets.

- The cli app will act as a remote control for the daemon. It connect's' to it and fetch stats or execute operations

- It support's the following operations:

   - status - Display the status of all the applications defined in the config file example output.:

     app1: running_time: 10hrs, running:true.....

     app2: running_time: 0, running:false....

     app3: running_time: 1hr, running:true

   - stop params:app_name - Will try to stop a running application gracefully and if can't will do it forcefully the app will not get auto started anymore even if autostart=true in this case.

   - start params:app_name - Will try to start an application that is not running. It should try endlessly until it succeeds.

   - restart params:app_name - Will first try to execute a stop and then start.

The project utilizes Tokio for async operations.

## Logging
The daemon creates/appends stdout to ./daemon.out and stderr to ./daemon.err files.
The apps generate their own out and err files in their specific folders.

## Docker
The code has a Dockerfile that packages the application to be run in a Docker. The image can be built using the following command:
```
docker build -f ~/rust_daemon/Dockerfile -t rust_daemon_app . --progress=plain --no-cache
```

The image can be run:
```
docker run -it rust_daemon_app
```
NOTE: The way the Dockerfile is constructed the docker run command directly puts you into the running container.

The runing container can be attached to with the following command:
```
docker exec -it <container_id> bash
```
