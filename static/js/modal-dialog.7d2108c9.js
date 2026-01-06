"use strict";(()=>{var i=class extends HTMLElement{connectedCallback(){let n=this.getAttribute("title")||"",t=this.getAttribute("close-target")||"",r=this.getAttribute("max-width")||"max-w-md",c=this.innerHTML;this.innerHTML=`
            <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" id="modal-overlay">
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 ${r} w-full mx-4">
                    <div class="flex justify-between items-center mb-4">
                        <h2 class="text-2xl font-bold">${this.escapeHtml(n)}</h2>
                        <button
                            type="button"
                            class="modal-close-btn text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                            </svg>
                        </button>
                    </div>
                    <div class="modal-content">
                        ${c}
                    </div>
                </div>
            </div>
        `;let o=this.querySelector(".modal-close-btn");o&&t&&o.addEventListener("click",()=>{let e=document.getElementById(t);e&&(e.innerHTML="")});let s=this.querySelector("#modal-overlay");s&&t&&s.addEventListener("click",e=>{if(e.target===s){let l=document.getElementById(t);l&&(l.innerHTML="")}})}escapeHtml(n){let t=document.createElement("div");return t.textContent=n,t.innerHTML}};customElements.define("modal-dialog",i);})();
