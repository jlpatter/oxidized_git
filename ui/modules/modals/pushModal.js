import {emit} from "@tauri-apps/api/event";

export class PushModal {
    constructor(mainJS) {
        this.mainJS = mainJS;
    }

    setListeners() {

    }

    setEvents() {
        const self = this;
        $('#openPushModalBtn').click(() => {
            if (Object.hasOwn(self.mainJS.generalInfo, 'head_has_upstream') && self.mainJS.generalInfo['head_has_upstream'] === 'true') {
                $('#remoteSelect').hide();
            } else {
                $('#remoteSelect').show();
            }
            $('#forcePushCheckBox').prop('checked', false);
            $('#pushModal').modal('show');
        });

        $('#pushBtn').click(() => {
            self.mainJS.addProcessCount();
            // Note: By default, pushing will try to use the local branch's upstream first
            // instead of the selected remote from the front-end
            emit("push", {
                selectedRemote: $('#remoteSelect').val(),
                isForcePush: $('#forcePushCheckBox').is(':checked').toString(),
            }).then();
            $('#pushModal').modal('hide');
        });
    }

    updateRemoteInfo(options) {
        const $remoteSelect = $('#remoteSelect');
        $remoteSelect.empty();
        options.forEach((option) => {
            $remoteSelect.append(option);
        });
    }
}