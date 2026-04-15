(() => {
  const STORAGE_KEY = "pulse.sidebarCollapsed";

  function readInitialCollapsed() {
    const body = document.body;
    const initial = body.getAttribute("data-sidebar-initial");
    if (initial === "collapsed") return true;
    if (initial === "expanded") return false;

    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved === "1") return true;
      if (saved === "0") return false;
    } catch {
      // Ignore storage errors (private mode, etc.)
    }
    return false;
  }

  function applyCollapsed(collapsed) {
    document.body.classList.toggle("sidebar-collapsed", collapsed);

    const aria = collapsed ? "Expand sidebar" : "Collapse sidebar";
    document.querySelectorAll("[data-sidebar-toggle]").forEach((btn) => {
      btn.setAttribute("aria-label", aria);
      btn.setAttribute("aria-expanded", collapsed ? "false" : "true");
    });
  }

  document.addEventListener("DOMContentLoaded", () => {
    applyCollapsed(readInitialCollapsed());

    document.querySelectorAll("[data-sidebar-toggle]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const next = !document.body.classList.contains("sidebar-collapsed");
        applyCollapsed(next);
        try {
          localStorage.setItem(STORAGE_KEY, next ? "1" : "0");
        } catch {
          // Ignore storage errors.
        }
      });
    });
  });
})();
