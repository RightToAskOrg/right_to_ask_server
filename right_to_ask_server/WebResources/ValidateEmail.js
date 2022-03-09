"use strict";

// function taken from tweetnacl-util, by Dmitry Chestnykh and Devi Mandiri, public domain.
function decodeBase64(s) {
    var i, d = atob(s), b = new Uint8Array(d.length);
    for (i = 0; i < d.length; i++) b[i] = d.charCodeAt(i);
    return b;
}

// function taken from tweetnacl-util, by Dmitry Chestnykh and Devi Mandiri, public domain.
function encodeBase64(arr) {
    var i, s = [], len = arr.length;
    for (i = 0; i < len; i++) s.push(String.fromCharCode(arr[i]));
    return btoa(s.join(''));
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

function getWhy() {
    switch (document.getElementById("Purpose").value) {
        case "MP" : return {"AsMP":true};
        case "MPStaffer" : return  {"AsMP":false};
        case "Org" : return "AsOrg";
        case "revokeMP" : return {"RevokeMP" : document.getElementById("Revokee").value};
        case "revokeOrg" : return {"RevokeOrg" : document.getElementById("Revokee").value};
    }
    failure("Need to say why");
    return null;
}

let email_id = null;

function RequestEmail() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            email_id = result.Ok.message;
            status("Requested email successfully and verified signature. Email id "+email_id);
        } else {
            status("Tried request email. Got Error message "+result.Err);
        }
    }
    let command = {
        email : document.getElementById("Email").value,
        why : getWhy()
    }
    let message = JSON.stringify(command);
    let user = document.getElementById("UID").value;
    let privateKey = document.getElementById("PrivateKey").value;
    const privateKeyUint8Array = decodeBase64(privateKey);
    const messageUint8Array = (new TextEncoder()).encode(message);
    const signatureUint8Array = nacl.sign.detached(messageUint8Array,privateKeyUint8Array);
    console.log(signatureUint8Array);
    let signature = encodeBase64(signatureUint8Array);
    let signed_message = { message:message, user:user, signature:signature };
    getWebJSON("request_email_validation",success,failure,JSON.stringify(signed_message),"application/json")
}

function VerifyCode() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            status("Code verified successfully and verified signature. Bulletin board "+result.Ok.message);
        } else if (result.Err) {
            status("Tried request email. Got Error message "+result.Err);
        } else {
            status("Code seems to have verified successfully. No Bulletin board entry.")
        }
    }
    let command = {
        hash : email_id,
        code : parseInt(document.getElementById("Code").value,10)
    }
    let message = JSON.stringify(command);
    let user = document.getElementById("UID").value;
    let privateKey = document.getElementById("PrivateKey").value;
    const privateKeyUint8Array = decodeBase64(privateKey);
    const messageUint8Array = (new TextEncoder()).encode(message);
    const signatureUint8Array = nacl.sign.detached(messageUint8Array,privateKeyUint8Array);
    console.log(signatureUint8Array);
    let signature = encodeBase64(signatureUint8Array);
    let signed_message = { message:message, user:user, signature:signature };
    getWebJSON("email_proof",success,failure,JSON.stringify(signed_message),"application/json")
}

window.onload = function () {
    document.getElementById("RequestEmail").onclick = RequestEmail;
    document.getElementById("VerifyCode").onclick = VerifyCode;
    updateQuestionList();
}