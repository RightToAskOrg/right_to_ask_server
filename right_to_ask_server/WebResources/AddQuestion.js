"use strict";

function showQuestion(div,user) {
    const a = add(div,"a");
    a.innerText=user;
    a.href = "get_question?question_id="+encodeURIComponent(user);
}
function updateQuestionList() {
    function success(data) {
        console.log(data);
        const div = document.getElementById("QuestionList");
        removeAllChildElements(div);
        if (data.Ok) for (const line of data.Ok) showQuestion(add(div,"div"),line);
        else if (data.Err) div.innerText="Error : "+data.Err;
    }
    getWebJSON("get_question_list",success,failure);
}

// function taken from tweetnacl-util, by Dmitry Chestnykh and Devi Mandiri, public domain.
function decodeBase64(s) {
    var i, d = atob(s), b = new Uint8Array(d.length);
    for (i = 0; i < d.length; i++) b[i] = d.charCodeAt(i);
    return b;
}

function check_signature(signed) {
    const message = signed.message;
    const signature = signed.signature;
    function success(publicKey) {
        const messageUint8Array = (new TextEncoder()).encode(message);
        const signatureUint8Array = decodeBase64(signature);
        const publicKeyUint8Array = decodeBase64(publicKey);
        let verified = nacl.sign.detached.verify(messageUint8Array,signatureUint8Array,publicKeyUint8Array);
        status("Verified "+verified+" for "+signature+" as a signature for "+message+" public key "+publicKey);
        if (crypto && crypto.subtle) {
            // actually can't do anything useful
        }
    }
    getWebJSON("get_server_public_key_raw",success,failure)
}

function addQuestion() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            const decoded = JSON.parse(result.Ok.message);
            status("Added question successfully. Question id "+decoded.question_id+" Bulletin Board hash "+decoded.version+" signature "+result.Ok.signature);
        } else {
            status("Tried to add user. Got Error message "+result.Err);
        }
        updateQuestionList();
    }
    let command = {
        question_text : document.getElementById("QuestionText").value
    }
    let message = JSON.stringify(command);
    let user = document.getElementById("UID").value;
    let private_key = document.getElementById("PrivateKey").value;
    let signature = ""; // TODO.
    let signed_message = { message:message, user:user, signature:signature };
    getWebJSON("new_question",success,failure,JSON.stringify(signed_message),"application/json")
}

window.onload = function () {
    document.getElementById("Add").onclick = addQuestion;
    updateQuestionList();
}