# E2E Static App __N__

A static interface for the arrow-apps WebDriver spec. upstream-up.sh renders
one copy per `__N__` (1 to 3), so the spec has several arrow apps to open at
once without a second chat.

```arrow
schema: "arrow@v0"

metadata:
  name: "E2E Static App __N__"
  description: "Serves a static page as its interface while a long sleep keeps the method open."
  version: "1.0"

targets:
  "*":
    lifecycle:
      install:
        - type: run
          command: 'mkdir -p www && echo "<!doctype html><html><head><title>E2E Static App __N__</title></head><body><h1 id=app>E2E Static App __N__</h1></body></html>" > www/index.html'
          title: "Write the page"
          timeout: "30s"
      uninstall:
        - type: run
          command: "rm -rf www"
          title: "Remove the page"
          timeout: "30s"
      execute:
        - type: ui
          static: www
          title: "E2E Static App __N__"
        - type: run
          command: "sleep 3000"
          title: "Keep the interface open"
          timeout: "3100s"
      stop:
        - type: signal
          signal: graceful
          title: "Stop"
          timeout: "5s"
          exit_on_failure: false
```
