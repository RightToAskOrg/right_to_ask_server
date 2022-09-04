"use strict";

function showList(userList) {
    console.log(userList);
    if (userList.Ok) {
        let tbody = document.getElementById("QueryResult");
        removeAllChildElements(tbody);
        for (const user of userList.Ok) {
            const tr = add(tbody,"tr");
            add(tr,"td").innerText=user.uid;
            add(tr,"td").innerText=user.display_name;
            let badges = add(tr,"td");
            if (user.badges) for (const badge of user.badges) {
                add(badges,"span","badge_"+badge.badge).innerText=badge.name;
            }
        }
    } else if (userInfo.Err) failure("Error : "+userInfo.Err);
}

function doSearch() {
    getWebJSON(getURL("search_user",{search:document.getElementById("Query").value,badge:true}),showList,failure);
}
window.onload = function () {
    document.getElementById("Query").addEventListener("input",function(event) {
        doSearch();
    });
    doSearch();
}