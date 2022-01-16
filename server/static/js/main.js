function sanitize(text){
  return $("<div>").text(text).html();
}

function get_agent_icon(agent_type){
  switch (agent_type) {
    case 'linux': return '<i class="fab fa-linux"></i>';
    case 'windows': return '<i class="fab fa-windows"></i>';
    default: return '<i class="fas fa-question"></i>';
  }
}

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
                    <span class="agent-badge bg-primary float-right">5</span>
                  </li>
                  <li class="nav-item px-2 pt-2 pb-1">
                    <i class="fas fa-memory p-2 align-middle"></i> RAM
                    <span class="agent-badge bg-primary float-right">5</span>
                  </li>
                </ul>
              </div>
        
              <!-- /.card-body -->
            </div>
            <!-- /.card -->
          </div>
  `;
}

function delete_agent(guid){
  $.ajax({
    url: `/api/agent/${guid}`,
    method: "DELETE",
    success: function(agents, textStatus){
      iziToast.success({
        title: 'OK',
        message: `Agent ${guid} has been successfully deleted!`,
      });
      $(`#${guid}`).CardWidget("remove");
    },
    error: function(errMsg){
      iziToast.error({
        title: 'Error',
        message: errMsg.statusText,
      });
    }
  });
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
      ['<button>Ok</button>', function (instance, toast) {
        delete_agent(guid);
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
            message: 'Agent successfully created!',
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
}

function main(){
  iziToast.settings({
    timeout: 10000,
    resetOnHover: true,
    position: 'topRight',
    theme: 'dark',
    icon: 'material-icons',
    transitionIn: 'flipInX',
    transitionOut: 'flipOutX',
  });

  setup_modals();

  if (window.location.href.match("/$")){
    $.ajax({
      url: '/api/agents',
      success: function(agents, textStatus){
        $("#agents_total h3").text(agents.length);
        $("#agents_total .overlay").remove();

        var alive_agents = agents.filter(agent => agent.status == "alive");
        $("#agents_alive h3").text(alive_agents.length);
        $("#agents_alive .overlay").remove();
      }
    });
  } else if (window.location.href.match("/agents$")) {
    $.ajax({
      url: "/api/agents",
      success: function(agents, textStatus){
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
    });

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
}

$(main);
