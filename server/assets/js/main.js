import '@fortawesome/fontawesome-free/css/all.min.css'
import 'izitoast/dist/css/iziToast.min.css'
import 'admin-lte/dist/css/adminlte.min.css'
import 'datatables.net-bs4/css/dataTables.bootstrap4.min.css'
import '/assets/css/main.css'

import $ from 'jquery';
window.jQuery = $;
window.$ = $;

import 'bootstrap';
import 'admin-lte';
import 'izitoast';

function formatBytes(a,b=2,k=1024){let d=Math.floor(Math.log(a)/Math.log(k));return 0==a?"0 Bytes":parseFloat((a/Math.pow(k,d)).toFixed(Math.max(0,b)))+" "+["Bytes","KB","MB","GB","TB","PB","EB","ZB","YB"][d]}

function renderAnalyzeStatus(data, type){
  if (type === "display"){
    if (data === null) {
      return "no"
    } else {
      return "yes";
    }
  }
  return data;
}

function renderDate(data, type) {
  if (type === "display") {
    return new Date(data).toLocaleString();
  }
  return data;
}

function parsePromData(data, metric) {
  let pallet = ['#1b9e77', '#d95f02', '#7570b3', '#e7298a', '#66a61e', '#e6ab02', '#a6761d', '#666666'];
  return {
    datasets: data.map((series, idx) => {
      return {
        label: series.metric[metric],
        data: series.values,
        backgroundColor: "transparent",
        borderColor: pallet[idx % pallet.length],
        pointRadius: 1
      };
    })
  };
}

window.formatBytes = formatBytes;
window.renderAnalyzeStatus = renderAnalyzeStatus;
window.renderDate = renderDate;
window.parsePromData = parsePromData;

function setup_modals(){
  $("#modal-add-agent :submit").click(function(event){

    var modal = $("#modal-add-agent");
    var description = $(modal).find("#description").first().val();
    var agent_type = $(modal).find("#agent-type").first().val();
    var endpoint = $(modal).find("#endpoint").first().val();

    $.ajax({
      url: "/api/agent",
      method: "POST",
      data: JSON.stringify({
        "description": description,
        "agent_type": agent_type,
        "endpoint": endpoint
      }),
      contentType:"application/json; charset=utf-8",
      success: function(agent, textStatus){
        iziToast.success({
            title: 'OK',
            message: 'Agent successfully created!'
        });
      },
      error: function(errMsg){
        iziToast.error({
            title: 'Error',
            message: errMsg.statusText,
        });
      }
    });
    event.preventDefault();
    $(modal).modal('hide');
  });

  $("#modal-add-job :submit").click(function(event){
    var modal = $("#modal-add-job");
    var fd = new FormData();

    var name = modal.find("#name").first().val();
    var description = modal.find("#description").first().val();
    var agent_type = modal.find("#job-agent-type").first().val();
    if(agent_type == "linux")
      var image = modal.find("#image").first().val();
    else {
      var image = "";
    }
    var cpus = modal.find("#cpus").first().val();
    var ram = modal.find("#ram").first().val();
    var timeout = modal.find("#timeout").first().val();
    var target = modal.find("#upload-target")[0].files[0];
    var corpus = modal.find("#upload-corpus")[0].files[0];
    var crash_auto_analyze = modal.find("#crash-auto-analyze").is(":checked");
    var trim_corpus = modal.find("#trim-corpus").is(":checked");

    if (name.length)
      fd.append("name", name);
    if (description.length)
      fd.append("description", description);
    fd.append("agent-type", agent_type);
    if (agent_type == "linux" && image.length)
      fd.append("image", image);
    if (cpus.length)
      fd.append("cpus", cpus);
    if (ram.length)
      fd.append("ram", ram);
    if (timeout.length)
      fd.append("timeout", timeout);
    if (target)
      fd.append("target", target);
    if (corpus)
      fd.append("corpus", corpus);
    fd.append("crash-auto-analyze", crash_auto_analyze);

    $.ajax({
      url: "/api/job",
      method: "POST",
      cache: false,
      data: fd,
      processData: false,
      contentType: false,
      success: function(job, textStatus){
        $(modal).modal("hide");
        iziToast.success({
          title: "OK",
          message: "Job successfully created!",
        });
      },
      error: function(errMsg) {
        iziToast.error({
          title: errMsg.statusText,
          message: errMsg.responseText
        });
      }
    });
    event.preventDefault();
  });
}

function main(){
  iziToast.settings({
    timeout: 10000,
    resetOnHover: true,
    position: 'topRight',
    theme: 'dark',
    transitionIn: 'flipInX',
    transitionOut: 'flipOutX',
  });

  //Tried to it with CSS but failed, help me
  $("#job-agent-type").change(function(){
    if(this.value == "linux") {
      $(".docker-image").show("scale");
    } else {
      $(".docker-image").hide("scale");
    }
  });

  setup_modals();
}

$(main);
