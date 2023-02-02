"use strict";

function change_list(url) {
    const email = document.getElementById("email").value;
    function success(status) {
        if (status.hasOwnProperty("Ok")) {
            refreshList();
        } else failure(status.Err);
    }
    getWebJSON(url,success,failure,JSON.stringify({email:email}),"application/json");
}

function put_on_list() { change_list("put_on_do_not_email_list"); }
function take_off_list() { change_list("take_off_do_not_email_list"); }

function refreshList() {
    const div = document.getElementById("current_list");
    removeAllChildElements(div);
    div.innerText="Loading...";
    function success(list) {
        if (list.Ok) {
            removeAllChildElements(div);
            for (const email of list.Ok) {
                add(div,"p").innerText=email.email;
            }
        } else failure(list.Err);
    }
    getWebJSON("get_do_not_email_list",success,failure);
}


window.onload = function () {
    refreshList();
}