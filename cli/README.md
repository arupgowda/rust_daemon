# Daemon CLI controller
This app controls the ATPCO daemon by sending commands to the running daemon and displaying the response from the daemon.
The app uses a TCP socket on the localhost:8081 for IPC with the daemon. 

## It supports the following commands:
- start 
- stop
- status
- restart

All commands except status takes an additional argument indicating the app name.

NOTE: Only status is implemented and start partially implemented.

```
cargo run -- --command <start|stop|status|restart> --app <app_name>
```

The CLI app can be tested locally without the daemon running by using telnet in the following way:

```
telnet 127.0.0.1 8081
Trying 127.0.0.1...
Connected to 127.0.0.1.
Escape character is '^]'.
{"command":"start","app":"tcp"}success
Connection closed by foreign host.
```
In the above example we received the request from the CLI app upon the connection made from Telnet:
```
{"command":"start","app":"tcp"}
```
and we responded with a success which will then be printed by the CLI app and exit. 
