document.getElementById("selectVersion").innerHTML = "";
addVersion("latest");
for (var i = 0; i < window.DOCS_COMMON.versions.length; i++) {
    addVersion(window.DOCS_COMMON.versions[i]);
}

function addVersion(version) {
    var select = document.getElementById("selectVersion");
    var base = window.DOCS_THIS_PACKAGE.root + "../";

    var option = document.createElement("option");
    option.selected = version == window.DOCS_THIS_PACKAGE.version;
    option.value = base + version + `/${window.DOCS_THIS_PACKAGE.name}/index.html`;
    option.innerText = version;
    select.appendChild(option);
}

function changeVersion() {
    var value = document.getElementById("selectVersion").value;
    location.href = value;
}
