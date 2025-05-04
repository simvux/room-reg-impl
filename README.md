## Purpose

This backend is meant to be an in-place replacement for the recently lost servers of a gaming community. 

It does not rely on any code or infrastructure from the project that was shut-down. 

*This repository is not for hosting a dedicated game server, it's for hosting the actual lobby browser web service*

### Changing client API URL

To switch your client to use a different web url; visit the folder `<configuration-folder>/qt-config.ini` and change these fields (while the client *is not running*)
```
web_api_url\default=false
web_api_url=api.ynet-fun.xyz
```
Feel free to substitute the URL for your own if you decide to self-host. 

(Optional) If you want your client to use TLS, you need to substitute the default URL embedded in the executable instead. 

### Changing server API URL

Add the `--web-api-url https://api.ynet-fun.xyz` flag. 

## Self Host Information

Make sure you have a valid ron configuration file in the runtime folder. 
```rs
// config.ron
Config(
    port: 3000,
    timeout_seconds: 30, // never set this to below 20 if you want to support normal clients
    user_limits: {
        "ip-address": 4, // set the maximum amount of room allowed by IP
    },
)
```

The normal client will not respect redirects. So; things like cloudflare proxies and forced TLS can mess things up. 
