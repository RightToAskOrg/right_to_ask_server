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

/// check a signed signature from the server, and print the result to the status() function.
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

// sign an object, using user name from field UID and privateKey from field PrivateKey
function sign_message(command) {
    let message = JSON.stringify(command);
    let user = document.getElementById("UID").value;
    if (!user) user = document.getElementById("UID").innerText;
    let privateKey = document.getElementById("PrivateKey").value;
    const privateKeyUint8Array = decodeBase64(privateKey);
    const messageUint8Array = (new TextEncoder()).encode(message);
    const signatureUint8Array = nacl.sign.detached(messageUint8Array,privateKeyUint8Array);
    console.log(signatureUint8Array);
    let signature = encodeBase64(signatureUint8Array);
    return { message:message, user:user, signature:signature };
}