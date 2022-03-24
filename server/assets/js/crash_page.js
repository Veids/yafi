import Buffer from 'buffer';
window.Buffer = Buffer.Buffer;

import { hexdump } from '@gct256/hexdump';

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
