const pickerOpts = {
  types: [
    {
      description: "Rust",
      accept: {
        "text/plain": [".rs"],
      },
    },
  ],
  excludeAcceptAllOption: true,
  multiple: false,
};

export class FileHandle {
  constructor(handle) {
    this._handle = handle;
  }

  async read() {
    if ("getFile" in this._handle) {
      let file = await this._handle.getFile();
      return file.text();
    } else {
      let file = await new Promise((resolve) => this._handle.file(resolve));
      return await file.text();
    }
  }
}

export async function open() {
  let [handle] = await window.showOpenFilePicker(pickerOpts);
  if (handle.kind != "file") {
    throw "Not a file";
  }
  return new FileHandle(handle);
}

export class DirectoryHandle {
  /**
   * @param {FileSystemDirectoryEntry} handle 
   */
  constructor(handle) {
    this._handle = handle;
  }

  /** @type {FileSystemDirectoryEntry} */
  _handle

  /** @type {Map<string, { lastModifiedDate: Date, contents: string }>} */
  fileCache = new Map();

  async getFiles() {
    let files = [];
    for await (let [_name, handle] of this._handle.entries()) {
      if (handle.kind == "file" && handle.name.endsWith(".rs")) {
        files.push(handle);
      }
    }
    return files;
  }

  async load_files() {
    // Load all rs files in our directory
    let fileHandles = await this.getFiles();

    // Sanity check that there are rs files here
    if (!fileHandles.length) {
      alert('No Rust source files found in this directory')
      throw "No .rs files found";
    }

    let files = await Promise.all(fileHandles.map(f => f.getFile()));

    return await Promise.all(files.map(async (file) => {
      let contents

      let cached = this.fileCache.get(file.name)

      if (!cached || cached.lastModifiedDate != file.lastModifiedDate) {
        // Reload from disk if we don't have it or it's changed
        contents = new TextDecoder().decode(await file.arrayBuffer())
        this.fileCache.set(file.name, { lastModifiedDate: file.lastModifiedDate, contents })
      } else {
        contents = cached.contents
      }

      return {
        name: file.name,
        lastModified: file.lastModifiedDate.getTime(),
        contents
      }
    }))
  }
}

export async function open_directory() {
  let entry = await showDirectoryPicker({
    id: 'oort-code-directory',
    mode: 'read'
  });

  return new DirectoryHandle(entry);
}