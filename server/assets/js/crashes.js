import 'datatables.net-bs4';
import 'datatables.net-responsive-bs4';

function main(){
  var t = $("#crashes-table").DataTable({
    "responsive": true,
    "autoWidth": false,
    "ajax": {
      "url": `/api/crashes`,
      "dataSrc": ""
    },
    "columns": [
      { "data": "guid" },
      {
        "data": "collection_guid",
        "render": $.fn.dataTable.render.text()
      },
      {
        "data": "name",
        "render": $.fn.dataTable.render.text()
      },
      {
        "data": "analyzed",
        "render": $.fn.dataTable.render.text()
      },
    ],
  });

  $(t.table().container()).on("click", "tbody tr", function(){
    var row = t.row(this);
    window.location = "/crash/" + row.data().guid;
  });
}

$(main);
