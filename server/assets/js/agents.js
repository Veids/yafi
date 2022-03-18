import $ from 'jquery';
window.jQuery = $;
window.$ = $;

import 'izitoast';
import 'dompurify';

function sanitize(text){
  return DOMPurify.sanitize(text);
}

function get_agent_icon(agent_type){
  switch (agent_type) {
    case 'linux': return '<i class="fab fa-linux"></i>';
    case 'windows': return '<i class="fab fa-windows"></i>';
    default: return '<i class="fas fa-question"></i>';
  }
}

function formatBytes(a,b=2,k=1024){let d=Math.floor(Math.log(a)/Math.log(k));return 0==a?"0 Bytes":parseFloat((a/Math.pow(k,d)).toFixed(Math.max(0,b)))+" "+["Bytes","KB","MB","GB","TB","PB","EB","ZB","YB"][d]}

function build_agent_box(agent){
  return `
        <div class="col-md-3">
            <div class="card ${agent.status == 'up' ? 'card-primary' : 'card-secondary'} card-outline" id="${agent.guid}">
              <div class="card-header">
                <h3 class="card-title">${get_agent_icon(agent.agent_type)} ${sanitize(agent.description)}</h3>
                <div class="card-tools">
                  <button type="button" class="btn btn-tool" data-card-widget="collapse">
                    <i class="fas fa-minus"></i>
                  </button>
                  <button type="button" class="btn btn-tool" data-agent-guid="${agent.guid}">
                    <i class="fas fa-times"></i>
                  </button>
                </div>
                <!-- /.card-tools -->
              </div>
              <!-- /.card-header -->
              <div class="card-body" style="display: block;">
                <ul class="nav nav-pills flex-column">
                  <li class="nav-item p-2">
                    <i class="fas fa-fingerprint p-2 align-middle"></i> GUID
                    <span class="agent-badge float-right">${agent.guid}</span>
                  </li>
                  <li class="nav-item p-2">
                    <i class="far fa-question-circle p-2 align-middle"></i> Last status                    
                    <span class="agent-badge float-right">Image pulling</span>
                  </li>
                  <li class="nav-item p-2">
                    <i class="fas fa-globe p-2 align-middle"></i> Endpoint
                    <span class="agent-badge float-right">${sanitize(agent.endpoint)}</span></a>
                  </li>
                  <li class="nav-item p-2">
                    <i class="fas fa-microchip p-2 align-middle"></i> CPUs
                    <span class="agent-badge bg-primary float-right">${agent.free_cpus}/${agent.cpus}</span>
                  </li>
                  <li class="nav-item px-2 pt-2 pb-1">
                    <i class="fas fa-memory p-2 align-middle"></i> RAM
                    <span class="agent-badge bg-primary float-right">${formatBytes(agent.free_ram * 1000)}/${formatBytes(agent.ram * 1000)}</span>
                  </li>
                </ul>
              </div>
        
              <!-- /.card-body -->
            </div>
            <!-- /.card -->
          </div>
  `;
}

function confirm_agent_delection(guid){
  iziToast.show({
    theme: 'dark',
    icon: 'icon-person',
    title: 'Confirm',
    message: `Agent ${guid} is being deleted!`,
    position: 'center',
    progressBarColor: 'rgb(0, 255, 184)',
    buttons: [
      ['<button>Ok</button>', async function (instance, toast) {
        await delete_agent(guid);
        instance.hide({
          transitionOut: 'fadeOutUp'
        }, toast);
      }, true], // true to focus
      ['<button>Close</button>', function (instance, toast) {
        instance.hide({
          transitionOut: 'fadeOutUp'
        }, toast);
      }]
    ],
  });
}

async function delete_agent(guid){
  try {
    const response = await fetch(`/api/agent/${guid}`, {
      method: "DELETE"
    });

    iziToast.success({
      title: 'OK',
      message: `Agent ${guid} has been successfully deleted!`,
    });
    $(`#${guid}`).CardWidget("remove");
  } catch (err) {
    iziToast.error({
      title: 'Error',
      message: err.statusText,
    });
  }
}

async function init_agents(){
  const response = await fetch("/api/agents");
  const agents = await response.json();

  $("#content-panel .overlay").remove();
  agents.forEach(agent => {
    var agent_box = $(build_agent_box(agent));
    var button = agent_box.find(":button[data-agent-guid]").first();
    button.click(event => {
      var guid = button.data("agent-guid");
      confirm_agent_delection(guid);
    });
    agent_box.appendTo("#content-panel");
  });
}

function init_search(){
  $("#search").submit(function(event){
    event.preventDefault();
    var string = $("#searchString").first().val();
    var filter = string.toLowerCase();
    $("#content-panel .info-box").each(function(index, element) {
      var desc = element.find("#description").text();
      if(desc.toLowerCase().indexOf(filter) > -1){
        element.style.display = "";
      } else {
        element.style.display = "none";
      }
    });
  });
}

async function main(){
  await Promise.all([init_agents()]);
}

(async() => {
  await main()  
})();
