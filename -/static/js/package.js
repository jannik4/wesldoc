// Show/hide package menu
function showPackageMenu(show) {
    const showPackage = document.getElementById("show-package");
    const packageMenu = document.getElementById("package-menu");

    showPackage.children[0].classList.remove("icon-down");
    showPackage.children[0].classList.remove("icon-up");
    showPackage.children[0].classList.add(show ? "icon-up" : "icon-down");

    packageMenu.style.display = show ? "grid" : "none";
}
document.getElementById("show-package").addEventListener("click", function (event) {
    event.preventDefault();
    const packageMenu = document.getElementById("package-menu");
    const isVisible = window.getComputedStyle(packageMenu).display !== "none";
    showPackageMenu(!isVisible);
});
window.addEventListener("click", function (event) {
    const showPackage = document.getElementById("show-package");
    const packageMenu = document.getElementById("package-menu");
    if (showPackage.contains(event.target) || packageMenu.contains(event.target)) {
        return;
    }
    showPackageMenu(false);
});

// Populate package versions
addPackageVersion("latest");
for (var i = 0; i < window.DOCS_COMMON.versions.length; i++) {
    addPackageVersion(window.DOCS_COMMON.versions[i]);
}

function addPackageVersion(version) {
    var div = document.getElementById("package-versions-list");
    var base = window.DOCS_THIS_PACKAGE.root + "../";

    var link = document.createElement("a");
    link.className = "muted-link";
    link.href = base + version + `/${window.DOCS_THIS_PACKAGE.name}/index.html`;
    link.innerText = version;
    div.appendChild(link);
}

