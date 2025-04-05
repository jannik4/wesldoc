function currentTheme() {
    const userPrefersDarkTheme = window.matchMedia("(prefers-color-scheme: dark)");
    return localStorage.getItem("DOCS_theme") || (userPrefersDarkTheme.matches ? "dark" : "light");
}

function updatePage(theme) {
    document.querySelector("html").setAttribute("data-theme", theme);
    const button = document.getElementById("toggleTheme");
    button.textContent = theme === "dark" ? "🌣" : "☾";
    button.setAttribute("aria-label", theme === "dark" ? "Switch to light mode" : "Switch to dark mode");
}

function toggleTheme() {
    const newTheme = currentTheme() === "dark" ? "light" : "dark";
    localStorage.setItem("DOCS_theme", newTheme);
    updatePage(newTheme);
}

updatePage(currentTheme());

// Update on show, e.g. when navigating back to the page
addEventListener("pageshow", () => { updatePage(currentTheme()); });
