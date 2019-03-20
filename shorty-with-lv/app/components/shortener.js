import Component from "@ember/component";
import { inject as service } from "@ember/service";
import ENV from "with-lv/config/environment";
import $ from 'jquery'

export default Component.extend({

  withLv: service(),

  init() {
    this._super(...arguments);
    this.prepareUIVarsBeforeShortening();
    this.set("shorteningResults", {});
  },

  didRender() {
    this._super(...arguments);
    $("input.url").focus();
  },

  prepareURL(url) {
    if (!url.toLowerCase().startsWith("http")) {
      url = `http://${url}`;
    }

    return url;
  },

  isValidURL(url) {
    try {
      new URL(url);
      return true;
    } catch (e) {
      return false;
    }
  },

  prepareUIVarsBeforeShortening() {
    this.set("invalidURL", null);
    this.set("genericError", null);
    this.set("shorteningResult", null);
  },

  actions: {
    shorten() {
      this.prepareUIVarsBeforeShortening();

      if (!this.get("url")) {
        return;
      }

      const previousResult = this.get("shorteningResults")[this.get("url")];
      if (previousResult) {
        this.set("shorteningResult", previousResult);
        return;
      }

      this.set("url", this.prepareURL(this.get("url")));

      if (!this.isValidURL(this.get("url"))) {
        this.set("invalidURL", this.get("url"));
        return;
      }

      this.get("shorteningResults")[this.get("url")] = true;

      this.withLv.shorten(this.get("url"))
        .then(result => {
          result.shortURL = `${ENV.APP.WITH_LV}/${result.id}`;
          this.get("shorteningResults")[this.get("url")] = result;
          this.set("shorteningResult", result);
        })
        .catch(err => {
          delete this.get("shorteningResults")[this.get("url")];
          this.set("genericError", err.responseJSON ? err.responseJSON.err : err.statusText);
        });
    }
  }

});
