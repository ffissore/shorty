import Service from "@ember/service";
import $ from "jquery"
import ENV from "with-lv/config/environment";

export default Service.extend({

  shorten(url) {
    return new Promise((resolve, reject) => {
      $.ajax(ENV.APP.WITH_LV, {
        method: "POST",
        contentType: "application/json",
        data: JSON.stringify({ url }),
        dataType: "json",
        success: (result) => {
          resolve(result);
        },
        error: (jqXHR) => {
          reject(jqXHR);
        }
      });
    });
  }

});
