function currentTheme() {
    return localStorage.getItem("DOCS_theme") || "auto";
}

function updatePage(theme) {
    const inputElement = document.getElementById("theme-" + theme);
    if (inputElement) {
        inputElement.checked = true;
    }

    if (theme === "auto") {
        const userPrefersDarkTheme = window.matchMedia("(prefers-color-scheme: dark)");
        theme = userPrefersDarkTheme.matches ? "dark" : "light";
    }
    document.querySelector("html").setAttribute("data-theme", theme);
}

updatePage(currentTheme());

// Update on show, e.g. when navigating back to the page
window.addEventListener("pageshow", () => { updatePage(currentTheme()); });

window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", event => {
    updatePage(currentTheme());
});

window.addEventListener("load", function () {
    document.getElementById("show-settings").addEventListener("click", function () {
        const settingsMenu = document.getElementById("settings-menu");
        const isVisible = window.getComputedStyle(settingsMenu).display !== "none";
        settingsMenu.style.display = isVisible ? "none" : "block";
    });
    window.addEventListener("click", function (event) {
        const settingsMenu = document.getElementById("settings-menu");
        if (settingsMenu.contains(event.target) || event.target.id === "show-settings") {
            return;
        }
        settingsMenu.style.display = "none";
    });

    const inputs = ["auto", "light", "dark"];
    for (const input of inputs) {
        document.getElementById("theme-" + input).addEventListener("click", function () {
            localStorage.setItem("DOCS_theme", input);
            updatePage(input);
        });
    }
});
