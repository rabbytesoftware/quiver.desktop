# E2E Updatable

An installed arrow with an `update` method, for the QOL spec: it has to be updatable
from the details page, and its page has to be long enough to scroll.

Section 1. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 2. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 3. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 4. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 5. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 6. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 7. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 8. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 9. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 10. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 11. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 12. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 13. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 14. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 15. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 16. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 17. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 18. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 19. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 20. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 21. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 22. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 23. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 24. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 25. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 26. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 27. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 28. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 29. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 30. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 31. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 32. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 33. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 34. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 35. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 36. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 37. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 38. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 39. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

Section 40. This paragraph exists only to make the arrow's details page taller than the window, so the QOL spec has a screen that really scrolls. It says nothing about the arrow.

```arrow
schema: "arrow@v0"

metadata:
  name: "E2E Updatable"
  description: "A small arrow the QOL spec installs, then updates from its details page."
  version: "1.0"

targets:
  "*":
    lifecycle:
      install:
        - type: run
          command: "echo installed"
          title: "Install"
          timeout: "30s"
      update:
        - type: run
          command: "echo updated"
          title: "Update"
          timeout: "30s"
          exit_on_failure: true
      uninstall:
        - type: run
          command: "echo removed"
          title: "Uninstall"
          timeout: "30s"
```
