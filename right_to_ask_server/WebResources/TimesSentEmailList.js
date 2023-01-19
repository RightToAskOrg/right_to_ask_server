"use strict";

function reset_times_sent(timescale) {
    console.log(timescale);
    function success(status) {
        if (status.hasOwnProperty("Ok")) {
            refreshList(timescale);
        } else failure(status.Err);
    }
    getWebJSON("reset_times_sent",success,failure,JSON.stringify(timescale),"application/json");
}

function remove_specific_email() {
    const email = document.getElementById("email").value;
    function success(status) {
        if (status.hasOwnProperty("Ok")) {
            refreshList(0);
            refreshList(1);
        } else failure(status.Err);
    }
    getWebJSON("take_off_times_sent_list",success,failure,JSON.stringify({email:email}),"application/json");
}

function refreshList(timescale) {
    const div = document.getElementById("current_list_"+timescale);
    removeAllChildElements(div);
    div.innerText="Loading...";
    function success(list) {
        if (list.Ok) {
            removeAllChildElements(div);
            for (const email of list.Ok) {
                add(div,"p").innerText=email.email+" : "+email.sent;
            }
        } else failure(list.Err);
    }
    getWebJSON(getURL("get_times_sent",{timescale:timescale}),success,failure);
}


window.onload = function () {
    refreshList(0);
    refreshList(1);
}