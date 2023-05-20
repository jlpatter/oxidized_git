import {emit} from "@tauri-apps/api/event";

export class PushTagModal {
    constructor(mainJS) {
        this.mainJS = mainJS;
    }

    setListeners() {

    }

    setEvents() {
        const self = this;
        $('#pushTagBtn').click(() => {
            self.mainJS.addProcessCount();
            const $tagName = $('#tagName');
            emit("push-tag", {
                tagFullName: $tagName.text(),
                selectedRemote: $('#remoteTagSelect').val(),
                isForcePush: $('#forcePushTagCheckBox').is(':checked').toString(),
            }).then();
            $tagName.text('');
            $('#pushTagModal').modal('hide');
        });
    }

    updateRemoteInfo(options) {
        const $remoteTagSelect = $('#remoteTagSelect');
        $remoteTagSelect.empty();
        options.forEach((option) => {
            $remoteTagSelect.append(option);
        });
    }
}
