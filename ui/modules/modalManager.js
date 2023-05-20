export class ModalManager {
    constructor(mainJS) {
        this.mainJS = mainJS;

        this.modals = [];
        this.pushModal = null;
        this.pushTagModal = null;
    }

    async loadModules() {
        const pushModalModule = await import("./modals/pushModal.js");
        this.pushModal = new pushModalModule.PushModal(this.mainJS);
        this.modals.push(this.pushModal);

        const pushTagModalModule = await import("./modals/pushTagModal.js");
        this.pushTagModal = new pushTagModalModule.PushTagModal(this.mainJS);
        this.modals.push(this.pushTagModal);
    }

    setListeners() {
        this.modals.forEach((m) => {
            m.setListeners();
        });
    }

    setEvents() {
        this.modals.forEach((m) => {
            m.setEvents();
        });
    }
}