function formatBytes(a,b=2,k=1024){with(Math){let d=floor(log(a)/log(k));return 0==a?"0 Bytes":parseFloat((a/pow(k,d)).toFixed(max(0,b)))+" "+["Bytes","KB","MB","GB","TB","PB","EB","ZB","YB"][d]}}

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

function build_job_row(job){
  return `
    <tr data-widget="expandable-table" aria-expanded="false">
      <td class="text-center">${job.agent_guid}</td>
    </tr>
    <tr class="expandable-body d-none">
      <td>
        <div class="container">
          <div class="container">
            <strong>
              <i class="fas fa-fingerprint"></i> Collection GUID
            </strong>
            <p class="text-muted mb-0">${job.collection_guid}</p>
          </div>
          <hr class="my-2">
          <div class="container">
            <strong>
              <i class="fas fa-star"></i> Master
            </strong>
            <p class="text-muted mb-0">${job.master}</p>
          </div>
          <hr class="my-2">
          <div class="container row">
            <div class="col-md-6 border-right">
              <strong class="align-middle">
                <i class="fas fa-microchip"></i> CPUs
              </strong>
              <span class="agent-badge float-right">${job.cpus}</span>
            </div>
            <div class="col-md-6">
              <strong class="align-middle">
                <i class="fas fa-memory"></i> RAM
              </strong>
              <span class="agent-badge float-right">${job.ram}</span>
            </div>
          </div>
          <hr class="my-2">
          <div class="container">
            <strong>
              <i class="fas fa-sticky-note"></i> Last message
            </strong>
            <p class="text-muted mb-0">${job.last_msg}</p>
          </div>
          <hr class="my-2">
          <div class="container">
            <strong>
              <i class="far fa-lightbulb"></i> Status
            </strong>
            <p class="text-muted mb-0">${job.status}</p>
          </div>
        </div>
      </td>
    </tr>
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

function job_stop(event, guid) {
  $.ajax({
    url: `/api/job/${guid}/stop`,
    success: function(textStatus) {
      iziToast.success({
        title: 'OK',
        message: 'Job stop request sent!',
      });
    },
    error: function(errMsg) {
      iziToast.error({
        title: 'Error',
        message: errMsg.statusText,
      });
    }
  });
  event.preventDefault();
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
    // icon: 'material-icons',
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

  if (window.location.pathname.match("/$")){
    $.ajax({
      url: '/api/agents',
      success: function(agents, textStatus){
        $("#agents_total h3").text(agents.length);
        $("#agents_total .overlay").remove();

        var alive_agents = agents.filter(agent => agent.status == "up");
        $("#agents_alive h3").text(alive_agents.length);
        $("#agents_alive .overlay").remove();
      }
    });
    $.ajax({
      url: '/api/job',
      success: function(stats, textStatus){
        $("#jobs_alive h3").text(stats.alive);
        $("#jobs_alive .overlay").remove();

        $("#jobs_completed h3").text(stats.completed);
        $("#jobs_completed .overlay").remove();

        $("#jobs_error h3").text(stats.error);
        $("#jobs_error .overlay").remove();
      }
    });
  } else if (window.location.pathname.match("/agents$")) {
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
  } else if (window.location.pathname.match("/jobs$")) {
    function renderBadge(data, type) {
      if (type === "display"){
        return `<span class="badge badge-secondary">${data}</span>`;
      }
      return data;
    }

    function renderAgentType(data, type) {
      if (type === "display"){
        var icon = "";
        switch (data) {
          case "linux":
            icon = "fab fa-linux";
            break;
          case "windows":
            icon = "fab fa-windows";
            break;
        }
        return `<i class="${icon}"></i> ${data}`;
      }
      return data;
    }

    function renderJobStatus(data, type) {
      if (type === "display") {
        var badge = "badge-secondary";

        switch (data){
          case "init":
          case "alive":
            badge = "badge-primary";
            break;
          case "completed":
            badge = "badge-success";
            break;
          case "error":
            badge = "badge-danger";
            break;
        }
        return `<span class="badge ${badge}">${data}</span>`;
      }
      return data;
    }

    // var t = $("#jobs-table").DataTable();
    var t = $("#jobs-table").DataTable({
      "responsive": true,
      "autoWidth": false,
      // "lengthChange": false,
      // "dom": "ftip",
      "ajax": {
        "url": "api/jobs",
        "dataSrc": ""
      },
      "columns": [
        { "data": "guid" },
        {
          "data": "name",
          "render": $.fn.dataTable.render.text()
        },
        {
         "data": "description",
          "render": $.fn.dataTable.render.text()
        },
        { "data": "creation_date" },
        { 
          "data": "agent_type",
          "className": "text-center",
          "render": renderAgentType
        },
        { 
          "data": "cpus",
          "className": "text-center",
          "render": renderBadge
        },
        { 
          "data": "ram",
          "className": "text-center",
          "render": renderBadge
        },
        { 
          "data": "timeout",
          "className": "text-center",
          "render": $.fn.dataTable.render.text()
        },
        { 
          "data": "status",
          "className": "text-center",
          "render": renderJobStatus
        },
      ],
      "order": [[3, "desc"]]
    });

    $(t.table().container()).on("click", "tbody tr", function(){
      var row = t.row(this);
      window.location = "job/" + row.data().guid;
    });
  } else if (window.location.pathname.match("/job/.+$")) {
    var guid = window.location.pathname.split("/").pop();
    $.ajax({
      url: `/api/job/${guid}`,
      success: function(job, textStatus){
        var card = $("#job-info");
        card.find("#name").text(job.job_collection.name);
        card.find("#description").text(job.job_collection.description);
        card.find("#guid").text(job.job_collection.guid);
        card.find("#created").text(job.job_collection.creation_date);
        card.find("#cpus").text(job.job_collection.cpus);
        card.find("#ram").text(job.job_collection.ram);
        card.find("#timeout").text(job.job_collection.timeout);
        card.find("#status").text(job.job_collection.status);
        if (job.job_collection.status == "alive" || job.job_collection.status == "init") {
          var stop = card.find("#stop");
          stop.click(function(event){
            job_stop(event, guid);
          });
          stop.show();
        }
        card.find(".overlay").remove();

        var assigned_agents = $("#assigned-agents tbody"); 
        job.jobs.forEach(job => {
          var tr = $(build_job_row(job));
          tr.appendTo(assigned_agents);
          tr.ExpandableTable("init");
        });
      },
      error: function(errMsg){
        iziToast.error({
          title: errMsg.statusText,
          message: errMsg.responseText
        });
      }
    });
  }
}

$(main);
