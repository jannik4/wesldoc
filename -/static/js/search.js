window.DOCS_INNER_CONTENT = document.getElementById("innerContent").innerHTML;

onUrlChange();
window.addEventListener('popstate', function (event) {
    onUrlChange();
});

function onUrlChange() {
    var query = document.getElementById("search").value;

    if ('URLSearchParams' in window) {
        var searchParams = new URLSearchParams(window.location.search);
        var param = searchParams.get("search") || "";
        if (param !== query) {
            document.getElementById("search").value = param;
            search();
        }
    }
}

window.addEventListener("keydown", function (event) {
    if (event.key === "s" && (event.target === document.body || event.target === null)) {
        event.preventDefault();
        var searchInput = document.getElementById("search");
        if (document.activeElement !== searchInput) {
            searchInput.focus();
        }
    }

    if (event.key === "Escape") {
        var searchInput = document.getElementById("search");
        if (event.target == searchInput) {
            event.preventDefault();
            searchInput.blur();
            searchInput.value = "";
            search();
        }
    }
});

function searchDebounced() {
    if (window.DOCS_SEARCH_DEBOUNCE) {
        clearTimeout(window.DOCS_SEARCH_DEBOUNCE);
    }
    window.DOCS_SEARCH_DEBOUNCE = setTimeout(search, 200);
}

function search() {
    var innerContentElement = document.getElementById("innerContent");
    var query = document.getElementById("search").value;
    var items = window.DOCS_ITEMS || [];

    if ('URLSearchParams' in window) {
        var searchParams = new URLSearchParams(window.location.search);
        if ((searchParams.get("search") || "") !== query) {
            if (query === "") {
                searchParams.delete("search");
            } else {
                searchParams.set("search", query);
            }

            var searchParamsString = searchParams.toString();
            if (searchParamsString === "") {
                history.pushState(null, '', window.location.pathname);
            } else {
                history.pushState(null, '', window.location.pathname + '?' + searchParamsString);
            }
        }
    }

    if (query === "") {
        innerContentElement.innerHTML = window.DOCS_INNER_CONTENT;
        return;
    }

    query = query.trim();
    var itemsFiltered = [];
    if (query !== "") {
        itemsFiltered = items.filter(function (item) {
            return item.name.toLowerCase().includes(query.toLowerCase())
                || item.attributes.some((attr) => attr.toLowerCase().startsWith(query.toLowerCase()));
        });
    }

    innerContentElement.innerHTML = "";
    var itemList = document.createElement("ul");
    itemList.className = "search-list";
    for (var i = 0; i < itemsFiltered.length; i++) {
        itemList.appendChild(createItemElement(itemsFiltered[i]));
    }
    innerContentElement.appendChild(itemList);
}

function createItemElement(item) {
    var className = "";
    switch (item.kind) {
        case "Module":
            className = "module";
            break;
        case "Constant":
            className = "const";
            break;
        case "Override":
            className = "override";
            break;
        case "GlobalVariable":
            className = "var";
            break;
        case "Struct":
            className = "struct";
            break;
        case "Function":
            className = "fn";
            break;
        case "TypeAlias":
            className = "type";
            break;
        default: break;
    }

    var firstDivElement = document.createElement("div");

    for (var i = 0; i < item.path.length; i++) {
        var segment = document.createElement("span");
        segment.className = "path";
        segment.innerText = item.path[i];
        firstDivElement.appendChild(segment);

        var segment = document.createElement("span");
        segment.className = "path";
        segment.innerText = "::";
        firstDivElement.appendChild(segment);

        firstDivElement.appendChild(document.createElement("wbr"));
    }

    var itemElement = document.createElement("span");
    itemElement.className = className;
    itemElement.innerText = item.name;
    firstDivElement.appendChild(itemElement);

    var secondDivElement = document.createElement("div");
    secondDivElement.innerHTML = item.comment;

    var linkElement = document.createElement("a");
    linkElement.href = window.DOCS_THIS_PACKAGE.root + item.url;
    linkElement.appendChild(firstDivElement);
    linkElement.appendChild(secondDivElement);

    return linkElement;
}
