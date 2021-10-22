"use strict";

function updatePending() {
    function success(data) {
        console.log(data);
        const div = document.getElementById("PendingList");
        removeAllChildElements(div);
        if (data.Ok) for (const line of data.Ok) addLink(add(div,"div"),line);
        else if (data.Err) div.innerText="Error : "+data.Err;
    }
    getWebJSON("get_parentless_unpublished_hash_values",success,failure);
}

function updatePublishedHead() {
    function success(data) {
        console.log(data);
        const div = document.getElementById("CurrentPublishedRoot");
        removeAllChildElements(div);
        if (data.Ok) addLink(div,data.Ok);
        else if (data.Err) div.innerText="Error : "+data.Err;
    }
    getWebJSON("get_most_recent_published_root",success,failure);
}

window.onload = function () {
    document.getElementById("DoMerkle").onclick = function () {
        function success(result) {
            console.log(result);
            if (result.Ok) {
                status("Made new published head "+result.Ok);
            } else status("Tried to make new published head, got error "+result.Err);
            updatePending();
            updatePublishedHead();
        }
        getWebJSON("order_new_published_root",success,failure,JSON.stringify(""),"application/json")
    }
    updatePending();
    updatePublishedHead();
}