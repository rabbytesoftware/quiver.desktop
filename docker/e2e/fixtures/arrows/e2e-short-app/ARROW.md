# E2E Short App

A static interface whose run step exits on its own after a few seconds, so
the arrow-apps spec can see the interface close with its process, the shell
return to the arrow's details, and the arrow start again.

```arrow
schema: "arrow@v0"

metadata:
  name: "E2E Short App"
  description: "Serves a static page for as long as a short sleep runs."
  version: "1.0"

targets:
  "*":
    lifecycle:
      install:
        - type: run
          command: 'mkdir -p www && echo "<!doctype html><html><head><title>E2E Short App</title></head><body><h1 id=app>E2E Short App</h1></body></html>" > www/index.html'
          title: "Write the page"
          timeout: "30s"
      uninstall:
        - type: run
          command: "rm -rf www"
          title: "Remove the page"
          timeout: "30s"
      execute:
        - type: run
          command: "sleep 12"
          title: "Keep the interface open for a while"
          timeout: "60s"
          ui:
            title: "E2E Short App"
            static: www
```
