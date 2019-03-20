import { module, test } from "qunit";
import { setupRenderingTest } from "ember-qunit";
import { render, find, fillIn, click } from "@ember/test-helpers";
import hbs from "htmlbars-inline-precompile";
import Service from '@ember/service';

module("Integration | Component | shortener", function(hooks) {
  setupRenderingTest(hooks);

  test("it shortens a url", async function(assert) {
    const withLvStub = Service.extend({
      shorten() {
        return Promise.resolve({ id: "123", url: "url" })
      }
    });
    this.owner.register('service:with-lv', withLvStub);

    await render(hbs`<Shortener/>`);

    assert.equal(find(".url").value.trim(), "");

    await fillIn(".url", "http://www.example.com");

    await click(".submit");

    assert.ok(find(".success").textContent.includes("http://with-lv/123 is your short url"));
  });

  test("it does't render an invalid url", async function(assert) {
    await render(hbs`<Shortener/>`);

    await fillIn(".url", "invalid url");

    await click(".submit");

    assert.equal(find(".invalidURL").textContent.trim(), "http://invalid url is not a valid URL");
  });

  test("it renders error", async function(assert) {
    const withLvStub = Service.extend({
      shorten() {
        return Promise.reject({ responseJSON: { err: "json error" } });
      }
    });
    this.owner.register('service:with-lv', withLvStub);

    await render(hbs`<Shortener/>`);

    await fillIn(".url", "http://www.example.com");

    await click(".submit");

    assert.equal(find(".genericError").textContent.trim(), "An error occurred: json error");
  });

  test("it calls the shortener service once for the same url", async function(assert) {
    const withLvStub = Service.extend({
      count: 0,
      shorten() {
        this.count++;

        if (this.count === 1) {
          return Promise.resolve({ id: "123", url: "url" });
        }

        throw new Error(`service called ${this.count} times`);
      }
    });
    this.owner.register('service:with-lv', withLvStub);

    await render(hbs`<Shortener/>`);

    await fillIn(".url", "http://www.example.com");

    await click(".submit");

    assert.ok(find(".success").textContent.includes("http://with-lv/123 is your short url"));

    await click(".submit");
  });
});
