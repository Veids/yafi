import Buffer from 'buffer';
window.Buffer = Buffer.Buffer;

import { hexdump } from '@gct256/hexdump';

function build_clusterfuzz_info(data){
  return `
    <p>Type: ${data.type}</p>
    <p>Is crash: ${data.is_crash}</p>
    <p>Is security issue: ${data.is_security_issue}</p>
    <p>Should ignore: ${data.should_ignore}</p>
    <p>Output:</p>
    <pre>${data.output}</pre>
    <p>Return code: ${data.return_code}</p>
    <p>Stacktrace:</p>
    <pre>${data.stacktrace}</pre>
  `;
}

function handle_clusterfuzz(data){
  var info = $("#clusterfuzz-info");
  info.find("#type").text(data.type || "unknown");
  info.find("#is-crash").text(data.is_crash);
  info.find("#is-security-issue").text(data.is_security_issue);
  info.find("#should-ignore").text(data.should_ignore);
  info.find("#output").text(data.output);
  info.find("#stacktrace").text(data.stacktrace);

  var nav = $('a[aria-controls="clusterfuzz"]');
  nav.removeClass("disabled");
}

async function init_crash_info(guid){
  try {
    const response = await fetch(`/api/crash/${guid}`);
    const crash = await response.json();

    var date = new Date(crash.creation_date);

    var card = $("#crash-info");
    card.find("#name").text(crash.name);
    card.find("#guid").text(crash.guid);
    card.find("#created").text(date.toLocaleString());
    card.find("#size").text(formatBytes(crash.size));
    card.find(".overlay").remove();

    $("#crash-hash").text(`sha256 - ${crash.hash}`);

    if (crash.analyzed !== null) {
      console.log(crash);
      var analyzed = JSON.parse(crash.analyzed);
      if (analyzed.clusterfuzz !== null) {
        handle_clusterfuzz(analyzed.clusterfuzz);
      }
    }
  } catch (err) {
    iziToast.error({
      title: 'Error',
      message: err.message,
    });
  }
}

async function init_hexdump(guid){
  try {
    var response = await fetch(`/api/crash/${guid}/get`);
    var data = await response.arrayBuffer();
    $("#hexdump-data").text(hexdump(data).join("\n"));
  } catch (err) {
    iziToast.error({
      title: 'Error',
      message: err.message,
    });
  }
}

async function main(){
  var guid = window.location.pathname.split("/").pop();
  await Promise.all([init_crash_info(guid), init_hexdump(guid)]);
}

(async() => {
  await main()
})();
