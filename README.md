# Bonsai
> Note:
- This is very WIP.
- Nothing is not finalized.

A small interpreted programming language.

<!-- haskell gives okay syntax highlighting -->
## Examples (Working)

- Sum till `n`
```haskell
let s = n -> {
  let x = 0
  loop {
    if (n == 0) 
      then { return x }
      else {
        x = x + n
        n = n - 1
      }
  }
}
print (s 20)

let s = n -> {
    if (n == 0)
      then (return 0)
      else (return (n + (s (n - 1))))
}
print (s 20)
```
- Factorial
```haskell
let f = n -> {
  if (n == 0)
    then (return 1)
    else (return (n * (f (n - 1))))
}
print (f 20)
```

## Examples (Thought Experiment)

- Basic syntax
```haskell
let x = 1

type Nat = x -> x:Int && x > 0

let factorial n = {
    let f = match n {
        0 -> 1
        _ -> n * factorial (n - 1)
    }

    match n {
        n : Nat - > f n
        _ -> throw (error "Natural number requried!")
    }
}
```

- A simple program using Server(Actor) based concurrency model.
```haskell
server Downloader {
  .init -> {
    self.jobs = hashmap.new () # There are no nullary functions
  }

  .download urls onDone -> {
    if len urls == 1 then {
      self (.downloadSingle (head urls) (.finish sender onDone))
    } else {
      let jobID = genToken () # Non forgable unique token
      self.jobs.set jobID (sender, onDone, urls, [])
      for u in urls {
        let worker = spawn Downloader
        worker (.donwloadSingle url (.collect jobID))
      }
    }
  }

  .downloadSingle url onDone -> {
    let file = wget url # Blocking
    sender (onDone url file)
  }

  .collect jobID url file -> {
    match self.jobs.get jobID {
      (requester, onDone, remaining, donwloaded) -> {
        let remaining = remove url remaining;
        let downloaded = add (url, file) downloaded;

        if len remaining == 0 {
          self.jobs.remove jobID
          requester (onDone downloaded)
        } else {
          self.jobs.set jobID (requester, onDone, remaining, downloaded)
        }
      }
    }
  }
}

server Main {
  .init args {
    let downloader = spawn Downloader
    downloader (.download args .exit)
  }

  .exit files {
    for (url, file) in files {
      let path = clean url # Get filename
      os.filesystem (.create path file)
    }
  }
}
```