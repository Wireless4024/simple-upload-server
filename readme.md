# Simple Upload Server
This is a very simple upload server but powered by [io_uring](https://en.wikipedia.org/wiki/Io_uring) to make sure every cpu clock is efficient while doing io task

## Requirement
+ Linux kernel version >= 5.1 
+ Rust (compile only)

## Configuration
```properties
# .env file

# ip:port to listen
BIND=0.0.0.0:8080
# upload code
CODE=
```
> If `CODE` is not set, will allow everyone to upload  
> else you need query param when upload like `http://localhost:8080/upload?code=<CODE_GOES_HERE>`