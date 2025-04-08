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

function searchDebounced() {
    if (window.DOCS_SEARCH_DEBOUNCE) {
        clearTimeout(window.DOCS_SEARCH_DEBOUNCE);
    }
    window.DOCS_SEARCH_DEBOUNCE = setTimeout(search, 400);
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

    var itemsFiltered = items.filter(function (item) {
        return item.name.toLowerCase().includes(query.toLowerCase())
            || item.attributes.some((attr) => attr.toLowerCase().startsWith(query.toLowerCase()));
    });

    innerContentElement.innerHTML = "";
    var itemList = document.createElement("ul");
    itemList.className = "item-list item-list-bordered";
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

    var linkElement = document.createElement("a");
    linkElement.href = window.DOCS_THIS_PACKAGE.root + item.url;

    for (var i = 0; i < item.path.length; i++) {
        var segment = document.createElement("span");
        segment.className = "path";
        segment.innerText = item.path[i];
        linkElement.appendChild(segment);

        var segment = document.createElement("span");
        segment.className = "path";
        segment.innerText = "::";
        linkElement.appendChild(segment);

        linkElement.appendChild(document.createElement("wbr"));
    }

    var itemElement = document.createElement("span");
    itemElement.className = className;
    itemElement.innerText = item.name;
    linkElement.appendChild(itemElement);

    var firstDivElement = document.createElement("div");
    firstDivElement.appendChild(linkElement);

    var secondDivElement = document.createElement("div");
    secondDivElement.innerHTML = item.comment;

    var listElement = document.createElement("li");
    listElement.appendChild(firstDivElement);
    listElement.appendChild(secondDivElement);

    return listElement;
}
