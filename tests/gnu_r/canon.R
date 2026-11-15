# Canonical text form of an R0 value. Must match `canon_r0` in tests/gnu_r.rs.
# Returns a character vector of lines.

canon_escape <- function(s) {
  s <- gsub("\\", "\\\\", s, fixed = TRUE)
  s <- gsub("\"", "\\\"", s, fixed = TRUE)
  s <- gsub("\n", "\\n", s, fixed = TRUE)
  paste0("\"", s, "\"")
}

canon_elem <- function(v, tt) {
  if (tt == "double") {
    if (is.nan(v)) return("NaN")
    if (is.na(v)) return("NA")
    if (is.infinite(v)) return(if (v > 0) "Inf" else "-Inf")
    return(sprintf("%.15g", v))
  }
  if (is.na(v)) return("NA")
  if (tt == "integer") return(sprintf("%d", v))
  if (tt == "logical") return(if (v) "TRUE" else "FALSE")
  canon_escape(v)
}

canon_container <- function(tag, x) {
  nms <- names(x)
  if (is.null(nms)) nms <- rep("", length(x))
  lines <- paste0(tag, "[", length(x), "]")
  for (i in seq_along(x)) {
    lines <- c(lines, paste0("  ", canon_escape(nms[i])))
    lines <- c(lines, paste0("    ", canon(x[[i]])))
  }
  lines
}

canon <- function(x) {
  if (is.null(x)) return("NULL")
  if (is.data.frame(x)) return(canon_container("data.frame", x))
  if (is.list(x)) return(canon_container("list", x))
  tt <- typeof(x)
  elems <- vapply(x, canon_elem, character(1), tt = tt, USE.NAMES = FALSE)
  paste(c(paste0(tt, "[", length(x), "]:"), elems), collapse = " ")
}
